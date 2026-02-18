use clap::{Parser, Subcommand};
use tracing_subscriber;
use std::fs;

mod domain;
mod application;
mod infrastructure;

use infrastructure::config::Config;
use infrastructure::adapters::telegram::TelegramAdapter;
use infrastructure::adapters::console::ConsoleAdapter;
use infrastructure::llm::{LLM, GroqProvider, LLMMessage};
use application::services::CommandService;
use domain::traits::Bot;

#[derive(Parser)]
#[command(name = "carik-bot")]
#[command(about = "A minimal secure bot framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// Bot token (overrides config)
    #[arg(short, long)]
    token: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the bot
    Run,
    /// Show version
    Version,
    /// Generate default config
    InitConfig,
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run => {
            run_bot(cli.config, cli.token);
        }
        Commands::Version => {
            println!("carik-bot v{}", env!("CARGO_PKG_VERSION"));
        }
        Commands::InitConfig => {
            init_config();
        }
    }
}

fn run_bot(config_path: String, token_override: Option<String>) {
    // Load config
    let config = if std::path::Path::new(&config_path).exists() {
        Config::load(&config_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}, using defaults", e);
            Config::load_env()
        })
    } else {
        Config::load_env()
    };

    tracing::info!("Starting carik-bot: {}", config.bot.name);

    // Initialize command service
    let mut commands = CommandService::new(&config.bot.prefix);
    commands.register_defaults();
    
    // Register workspace command
    register_workspace_command(&mut commands);
    
    // Register kiro command
    register_kiro_command(&mut commands);

    // Select adapter
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    if let Some(token) = token_override.or_else(|| {
        config.adapters.telegram
            .as_ref()
            .and_then(|t| t.token.clone())
    }) {
        // Run Telegram bot
        let allowed_users = if config.whitelist.enabled {
            Some(config.whitelist.users.clone())
        } else {
            None
        };
        rt.block_on(async {
            let mut bot = TelegramAdapter::new(token, allowed_users);
            
            // Register bot commands with Telegram
            if let Err(e) = bot.register_commands().await {
                tracing::warn!("Failed to register commands: {}", e);
            }
            
            run_telegram_bot(&mut bot, &mut commands).await;
        });
    } else {
        // Run console bot (dev mode)
        rt.block_on(async {
            let bot = ConsoleAdapter::new();
            run_console_bot(bot, commands).await;
        });
    }
}

async fn run_telegram_bot(bot: &mut TelegramAdapter, commands: &mut CommandService) {
    use domain::entities::{Message, Content};
    use infrastructure::adapters::telegram::TelegramAdapter;

    // Fetch bot info
    if let Err(e) = bot.fetch_bot_info().await {
        tracing::error!("Failed to fetch bot info: {}", e);
        return;
    }

    let info = bot.bot_info();
    tracing::info!("Bot started: @{}", info.username);

    // Load SOUL.md as system persona
    let system_prompt = match fs::read_to_string("SOUL.md") {
        Ok(content) => content,
        Err(_) => {
            tracing::warn!("SOUL.md not found, using default persona");
            "You are carik-bot, a helpful and friendly AI assistant.".to_string()
        }
    };
    tracing::info!("Loaded persona from SOUL.md");

    // Initialize LLM (Groq with Llama)
    let llm: Option<GroqProvider> = match std::env::var("GROQ_API_KEY") {
        Ok(api_key) => {
            Some(GroqProvider::new(api_key, Some("llama-3.1-8b-instant")))
        }
        Err(_) => {
            tracing::warn!("GROQ_API_KEY not set, using echo mode");
            None
        }
    };
    if llm.is_some() {
        tracing::info!("Using Groq Llama 3.1 8B for AI responses");
    }

    // Track first messages per chat for welcome
    let mut first_message: std::collections::HashMap<String, bool> = std::collections::HashMap::new();

    // Conversation history per chat
    let mut conversations: std::collections::HashMap<String, Vec<LLMMessage>> = std::collections::HashMap::new();

    let mut offset: i64 = 0;
    let timeout_seconds = 30;

    loop {
        match bot.get_updates(offset, timeout_seconds).await {
            Ok(updates) => {
                for update in &updates {
                    // Extract chat_id and text from message
                    if let Some(msg) = &update.message {
                        let chat_id = msg.chat.id.to_string();
                        let text = msg.text.clone().unwrap_or_default();
                        
                        if !text.is_empty() {
                            // Check if this is the first message from this chat
                            let is_first = first_message.get(&chat_id).is_none();
                            if is_first {
                                first_message.insert(chat_id.clone(), true);
                            }
                            
                            // Process command or message
                            let response = if text.starts_with(&commands.prefix()) || text.starts_with('/') {
                                // Check for /code command (coding agent)
                                let trimmed = text.trim_start_matches(&commands.prefix()).trim_start_matches('/');
                                if trimmed.starts_with("code") {
                                    // Extract the prompt from /code command
                                    let prompt = if trimmed.starts_with("code ") {
                                        // Has space after "code", grab everything after position 5
                                        trimmed[5..].trim().to_string()
                                    } else if trimmed.len() > 5 {
                                        // Has content after "code" (no space) - grab from position 4
                                        trimmed[4..].trim().to_string()
                                    } else {
                                        // Just "code" or "/code" with nothing after
                                        String::new()
                                    };
                                    
                                    if prompt.is_empty() {
                                        "Usage: /code <your coding task>\nExample: /code write a hello world in python".to_string()
                                    } else {
                                        // Execute kiro-cli as coding agent
                                        execute_kiro_cli(&prompt).await
                                    }
                                } else {
                                    let cmd_parts: Vec<&str> = trimmed.split_whitespace().collect();
                                    let cmd_name = cmd_parts.first().unwrap_or(&"").to_string();
                                    let args: Vec<String> = cmd_parts[1..].iter().map(|s| s.to_string()).collect();
                                    
                                    let msg = Message::from_command(&chat_id, cmd_name, args);
                                    match commands.handle(&msg) {
                                        Ok(Some(response)) => response,
                                        Ok(None) => continue,
                                        Err(e) => format!("Error: {}", e),
                                    }
                                }
                            } else if let Some(ref llm) = llm {
                                // Use LLM for conversation
                                let chat_history = conversations.entry(chat_id.clone()).or_insert_with(Vec::new);
                                
                                // Add user message to history
                                chat_history.push(LLMMessage::user(&text));
                                
                                // Build messages with system prompt
                                let mut messages = vec![LLMMessage::system(&system_prompt)];
                                messages.extend(chat_history.clone());
                                
                                // Get LLM response
                                match llm.chat(messages, None, Some(0.7), None).await {
                                    Ok(response) => {
                                        chat_history.push(LLMMessage::assistant(&response.content));
                                        
                                        // Limit history to last 10 messages
                                        if chat_history.len() > 10 {
                                            chat_history.remove(0);
                                        }
                                        
                                        // Add Javanese greeting on first message
                                        if is_first {
                                            let greeting = generate_javanese_greeting(&info.username);
                                            format!("{}\n\n{}", greeting, response.content)
                                        } else {
                                            response.content
                                        }
                                    }
                                    Err(e) => {
                                        format!("LLM Error: {}", e)
                                    }
                                }
                            } else {
                                // Echo mode when LLM is not available
                                format!("Echo: {}", text)
                            };

                            // Send response
                            tracing::info!("Sending response to chat_id {}: {}", chat_id, &response[..response.len().min(100)]);
                            if let Err(e) = bot.send_message(&chat_id, &response).await {
                                tracing::error!("Failed to send message: {}", e);
                            }
                        }
                    }
                    
                    // Handle callback queries
                    if let Some(cb) = &update.callback_query {
                        let chat_id = cb.message.as_ref().map(|m| m.chat.id.to_string()).unwrap_or_default();
                        if let Some(data) = &cb.data {
                            let _ = bot.send_message(&chat_id, &format!("Callback: {}", data)).await;
                        }
                        let _ = bot.answer_callback(&cb.id, None).await;
                    }
                }
                
                // Update offset
                offset = TelegramAdapter::get_next_offset(&updates);
            }
            Err(e) => {
                tracing::error!("Failed to get updates: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Generate Javanese-style greeting
/// Execute kiro-cli as a coding agent in the workspace directory
async fn execute_kiro_cli(prompt: &str) -> String {
    execute_kiro_cli_in_dir(prompt, &get_workspace_dir()).await
}

async fn execute_kiro_cli_in_dir(prompt: &str, dir: &std::path::Path) -> String {
    use tokio::process::Command;
    use std::io::Write;
    
    tracing::info!("Executing kiro-cli in {:?} with prompt: {}", dir, prompt);
    
    // Ensure workspace directory exists
    if !dir.exists() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            return format!("Error: Could not create workspace directory: {}", e);
        }
    }
    
    let mut child = Command::new("/home/ubuntu/.local/bin/kiro-cli")
        .arg("chat")
        .arg("--no-interactive")
        .arg("--trust-all-tools")
        .arg(&prompt)
        .current_dir(dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn kiro-cli");
    
    let output = child.wait_with_output().await.expect("Failed to wait for kiro-cli");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Combine and strip ANSI codes
    let mut combined = stdout.to_string();
    if !stderr.is_empty() {
        combined.push_str("\nStderr: ");
        combined.push_str(&stderr);
    }
    
    // Strip ANSI escape codes
    let cleaned = strip_ansi_codes(&combined);
    
    // Limit response length for Telegram
    let max_len = 4000;
    if cleaned.len() > max_len {
        format!("{}...\n\n(Output truncated, {} chars total)", &cleaned[..max_len], cleaned.len())
    } else if cleaned.trim().is_empty() {
        "No output from kiro-cli".to_string()
    } else {
        cleaned
    }
}

/// Strip ANSI escape codes from string
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip the escape sequence
            if chars.next() == Some('[') {
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphabetic() {
                        chars.next();
                        break;
                    } else {
                        chars.next();
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    
    // Clean up extra whitespace
    let cleaned = result.trim().to_string();
    cleaned.lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Workspace management
const CARIK_HOME: &str = "/home/ubuntu/.carik-bot";

fn register_workspace_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    commands.register(Command::new("workspace")
        .with_description("Manage workspaces")
        .with_usage("/workspace <list|create|delete|switch> [name]")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            let args_str = args.join(" ");
            let parts: Vec<&str> = args_str.split_whitespace().collect();
            
            let response = match parts.first().map(|s| *s) {
                Some("list") | Some("ls") | None => list_workspaces(),
                Some("create") | Some("new") => {
                    if parts.len() < 2 {
                        Ok("Usage: /workspace create <name>".to_string())
                    } else {
                        create_workspace(parts[1])
                    }
                }
                Some("delete") | Some("rm") => {
                    if parts.len() < 2 {
                        Ok("Usage: /workspace delete <name>".to_string())
                    } else {
                        delete_workspace(parts[1])
                    }
                }
                Some("switch") | Some("use") => {
                    if parts.len() < 2 {
                        Ok("Usage: /workspace switch <name>".to_string())
                    } else {
                        switch_workspace(parts[1])
                    }
                }
                Some("current") | Some("info") => get_current_workspace(),
                _ => Ok("Usage: /workspace <list|create|delete|switch|current> [name]".to_string())
            };
            
            response.map_err(|e| crate::application::errors::CommandError::ExecutionFailed(e))
        }));
}

fn list_workspaces() -> Result<String, String> {
    let home = std::path::Path::new(CARIK_HOME);
    if !home.exists() {
        std::fs::create_dir_all(home).map_err(|e| e.to_string())?;
    }
    
    let mut output = "Workspaces:\n".to_string();
    
    for entry in std::fs::read_dir(home).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() && path.file_name().map(|n| !n.to_string_lossy().starts_with('.')).unwrap_or(false) {
            let name = path.file_name().unwrap().to_string_lossy();
            output.push_str(&format!("  - {}\n", name));
        }
    }
    
    if output == "Workspaces:\n" {
        output.push_str("  (none)\n");
    }
    
    output.push_str("\nCurrent: default-workspace");
    Ok(output)
}

fn create_workspace(name: &str) -> Result<String, String> {
    // Validate name
    if name.contains(|c: char| c.is_whitespace() || c == '/' || c == '.') {
        return Ok("Invalid workspace name. Use alphanumeric and underscores only.".to_string());
    }
    
    let workspace_path = std::path::Path::new(CARIK_HOME).join(name);
    
    if workspace_path.exists() {
        return Ok(format!("Workspace '{}' already exists", name));
    }
    
    std::fs::create_dir_all(&workspace_path).map_err(|e| e.to_string())?;
    Ok(format!("Created workspace: {}", name))
}

fn delete_workspace(name: &str) -> Result<String, String> {
    if name == "default-workspace" {
        return Ok("Cannot delete default-workspace".to_string());
    }
    
    let workspace_path = std::path::Path::new(CARIK_HOME).join(name);
    
    if !workspace_path.exists() {
        return Ok(format!("Workspace '{}' does not exist", name));
    }
    
    std::fs::remove_dir_all(&workspace_path).map_err(|e| e.to_string())?;
    Ok(format!("Deleted workspace: {}", name))
}

fn switch_workspace(name: &str) -> Result<String, String> {
    let workspace_path = std::path::Path::new(CARIK_HOME).join(name);
    
    if !workspace_path.exists() {
        return Ok(format!("Workspace '{}' does not exist. Use /workspace create {} to create it.", name, name));
    }
    
    // For now, just acknowledge the switch (in a full implementation, would persist this)
    Ok(format!("Switched to workspace: {}", name))
}

fn get_current_workspace() -> Result<String, String> {
    Ok("Current workspace: default-workspace".to_string())
}

fn get_workspace_dir() -> std::path::PathBuf {
    std::path::Path::new(CARIK_HOME).join("default-workspace")
}

fn get_docker_workspace_dir() -> String {
    "/workspace/default-workspace".to_string()
}

/// Kiro CLI tmux session management
const KIRO_SOCKET: &str = "/tmp/carik-kiro.sock";
const KIRO_SESSION: &str = "carik-kiro";

fn register_kiro_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    // Main kiro command - handles /kiro <prompt>
    commands.register(Command::new("kiro")
        .with_description("Run kiro-cli in tmux")
        .with_usage("/kiro <prompt>")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            if args.is_empty() {
                return Ok("Usage: /kiro <prompt>\n/kiro-status - Check if running\n/kiro-log - See output\n/kiro-kill - Stop session".to_string());
            }
            
            // Join all args as the prompt
            let prompt = args.join(" ");
            kiro_start(&prompt).map_err(|e| crate::application::errors::CommandError::ExecutionFailed(e))
        }));
    
    // Subcommands
    commands.register(Command::new("kiro-status")
        .with_description("Check kiro status")
        .with_handler(|_| kiro_status().map_err(|e| crate::application::errors::CommandError::ExecutionFailed(e))));
    
    commands.register(Command::new("kiro-log")
        .with_description("Get kiro output")
        .with_handler(|_| kiro_log().map_err(|e| crate::application::errors::CommandError::ExecutionFailed(e))));
    
    commands.register(Command::new("kiro-kill")
        .with_description("Kill kiro session")
        .with_handler(|_| kiro_kill().map_err(|e| crate::application::errors::CommandError::ExecutionFailed(e))));
}

const KIRO_CONTAINER: &str = "kiro-persistent";

fn kiro_start(prompt: &str) -> Result<String, String> {
    // Check if container is running
    let check = std::process::Command::new("docker")
        
        .args(["inspect", "-f", "{{.State.Running}}", KIRO_CONTAINER])
        .output();
    
    let is_running = check
        .as_ref()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false);
    
    if is_running {
        // Container running - send prompt as argument
        let workspace_dir = get_docker_workspace_dir();
        let cmd = format!(
            "cd {} && kiro-cli chat --no-interactive --trust-all-tools {}",
            workspace_dir,
            prompt.replace("'", "'\\''")
        );
        
        let output = std::process::Command::new("docker")
        
            .args(["exec", KIRO_CONTAINER, "bash", "-c", &cmd])
            .output()
            .map_err(|e| e.to_string())?;
        
        if output.status.success() {
            let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
            tracing::info!("Kiro stdout length: {}", stdout.len());
            // Save output for kiro-log
            match std::fs::write("/tmp/kiro-last-output.txt", stdout.to_string()) {
                Ok(_) => tracing::info!("Output saved to file"),
                Err(e) => tracing::error!("Failed to save output: {}", e),
            }
            return Ok(format!("📤 Kiro response:\n{}\n\nUse /kiro-log for full output.", &stdout[..stdout.len().min(500)]));
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Kiro error: {}", err);
            let _ = std::fs::write("/tmp/kiro-last-output.txt", err.to_string());
            return Ok(format!("❌ Kiro error: {}", err));
        }
    }
    
    // Container not running - start it
    // Note: Container should already be started as a service
    // This is just a fallback
    Ok("Kiro container not running. Please restart the bot.".to_string())
}

fn kiro_status() -> Result<String, String> {
    let output = std::process::Command::new("docker")
        
        .args(["inspect", "-f", "{{.State.Running}}", KIRO_CONTAINER])
        .output()
        .map_err(|e| e.to_string())?;
    
    let is_running = output.status.success() && 
        String::from_utf8_lossy(&output.stdout).trim() == "true";
    
    if is_running {
        Ok("🟢 Kiro session is running (Docker)".to_string())
    } else {
        Ok("🔴 Kiro session is not running. Use /kiro-start to start.".to_string())
    }
}

fn kiro_log() -> Result<String, String> {
    // Try to read from stored output file first
    let output_file = "/tmp/kiro-last-output.txt";
    if let Ok(content) = std::fs::read_to_string(output_file) {
        if !content.is_empty() {
            return Ok(format!("📋 Last Kiro output:\n```\n{}```", &content[..content.len().min(3000)]));
        }
    }
    
    // Fallback to docker logs
    let output = std::process::Command::new("docker")
        
        .args(["logs", "--tail", "50", KIRO_CONTAINER])
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        let logs = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        if logs.is_empty() {
            return Ok("No output yet. Use /kiro <prompt> to start.".to_string());
        }
        Ok(format!("📋 Kiro logs:\n```\n{}```", &logs[..logs.len().min(2000)]))
    } else {
        Ok("No active Kiro session. Use /kiro <prompt> to start.".to_string())
    }
}

fn kiro_kill() -> Result<String, String> {
    let output = std::process::Command::new("docker")
        
        .args(["kill", KIRO_CONTAINER])
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok("🔴 Kiro session stopped.".to_string())
    } else {
        Ok("No session to stop.".to_string())
    }
}

// Helper to strip ANSI escape codes from kiro output
fn strip_ansi(s: &str) -> String {
    let re = regex_lite::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

fn generate_javanese_greeting(bot_username: &str) -> String {
    format!("Sugeng rawuh Pak Lurah Ing {}\n\nKulo niku Carik AI Assistant.\nNyuwun sewu, kepareng nepangaken.\nPanjenenganipun inggih punika tamu ing wewaton iki.\nMonggo kerso dipunbotenaken. Sendiko dawuh!\n\n/help - Pitulungan\n/about - Nepangaken Carik\n/ping - Mriki Piyambak\n/clear - Ngresikaken Obrolan\n/quote - UnggahQuote", bot_username)
}

async fn run_console_bot<B: Bot>(bot: B, mut commands: CommandService) {
    use domain::entities::Message;
    use domain::entities::Content;

    if let Err(e) = bot.start().await {
        tracing::error!("Failed to start bot: {}", e);
        return;
    }

    let info = bot.bot_info();
    tracing::info!("Bot started: @{}", info.username);

    // Main loop (for console mode)
    loop {
        if let Some(input) = ConsoleAdapter::new().read_line("> ").await {
            if input.is_empty() {
                continue;
            }

            // Check for commands
            let input = input.trim();
            if input.starts_with(&commands.prefix()) || input.starts_with('/') {
                let cmd_name = input.trim_start_matches(&commands.prefix())
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("");

                let msg = Message::from_command("console", cmd_name, vec![]);
                match commands.handle(&msg) {
                    Ok(Some(response)) => {
                        let _ = bot.send_message("console", &response).await;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let _ = bot.send_message("console", &format!("Error: {}", e)).await;
                    }
                }
            } else {
                // Echo mode
                let msg = Message::from_text("console", input);
                let _ = bot.send_message("console", &format!("Echo: {}", input)).await;
            }
        }
    }
}

fn init_config() {
    let config = Config::default();
    let yaml = serde_yaml::to_string(&config).unwrap();
    println!("{}", yaml);
    println!("\nSave this to config.yaml and adjust as needed.");
}
