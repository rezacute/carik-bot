use clap::{Parser, Subcommand};
use tracing_subscriber;
use std::fs;
use std::sync::Mutex;
use once_cell::sync::Lazy;

mod domain;
mod application;
mod infrastructure;
mod plugins;

use infrastructure::config::Config;
use infrastructure::database;
use infrastructure::adapters::telegram::TelegramAdapter;
use infrastructure::adapters::console::ConsoleAdapter;
use infrastructure::llm::{LLM, GroqProvider, LLMMessage};
use application::services::CommandService;
use domain::traits::Bot;
use plugins::{PluginManager, trait_def::ExtendedPluginConfig};

// Global database instance
static DB: Lazy<Mutex<Option<database::Database>>> = Lazy::new(|| Mutex::new(None));

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
    
    // Initialize database
    let db = match database::Database::new("carik-bot.db") {
        Ok(db) => {
            tracing::info!("Database initialized");
            // Set global DB
            *DB.lock().unwrap() = Some(db);
            Some(())
        }
        Err(e) => {
            tracing::error!("Failed to initialize database: {}", e);
            None
        }
    };
    
    // Initialize owner from config if not exists
    if DB.lock().unwrap().is_some() {
        if let Ok(config) = Config::load("config.yaml") {
            for user_id in &config.whitelist.users {
                let _ = DB.lock().unwrap().as_ref().unwrap().add_user(user_id, None, "owner");
            }
        }
    }
    
    // Initialize plugin system
    let plugin_config = ExtendedPluginConfig {
        enabled: true,
        plugins_dir: config.plugins.directory.to_str().map(|s| s.to_string()),
        mcp: None,  // Phase 2
        a2a: None,  // Phase 3
        wasm: None, // Phase 4
    };
    let mut plugin_manager = PluginManager::new(plugin_config);
    if let Err(e) = plugin_manager.load_from_config() {
        tracing::warn!("Failed to load plugins: {}", e);
    }
    tracing::info!("Plugin system initialized with {} plugins", plugin_manager.list_plugins().len());

    // Initialize command service
    let mut commands = CommandService::new(&config.bot.prefix);
    commands.register_defaults();
    
    // Register start command (welcome message)
    register_start_command(&mut commands);
    
    // Register connect command (for guests)
    register_connect_command(&mut commands);
    
    // Register approve command (owner only)
    register_approve_command(&mut commands);
    
    // Register users command (owner/admin)
    register_users_command(&mut commands);
    
    // Register workspace command
    register_workspace_command(&mut commands);
    
    // Register kiro command
    register_kiro_command(&mut commands);
    
    // Register RSS command
    register_rss_command(&mut commands);
    
    // Register settings command
    register_settings_command(&mut commands);

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

    // Initialize LLM (Groq with selected model)
    let llm: Option<GroqProvider> = match std::env::var("GROQ_API_KEY") {
        Ok(api_key) => {
            // Check for saved model preference
            let model = std::fs::read_to_string("/home/ubuntu/.carik-bot/groq-model.txt")
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());
            
            Some(GroqProvider::new(api_key, Some(&model)))
        }
        Err(_) => {
            tracing::warn!("GROQ_API_KEY not set, using echo mode");
            None
        }
    };
    if llm.is_some() {
        let model = std::fs::read_to_string("/home/ubuntu/.carik-bot/groq-model.txt")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());
        tracing::info!("Using Groq {} for AI responses", model);
    }

    // Track first messages per chat for welcome
    let mut first_message: std::collections::HashMap<String, bool> = std::collections::HashMap::new();

    // Conversation history per chat
    let mut conversations: std::collections::HashMap<String, Vec<LLMMessage>> = std::collections::HashMap::new();

    let mut offset: i64 = 0;
    let timeout_seconds = 30;

    tracing::info!("Starting message loop...");
    
    loop {
        match bot.get_updates(offset, timeout_seconds).await {
            Ok(updates) => {
                if !updates.is_empty() {
                    tracing::info!("Received {} updates", updates.len());
                }
                for update in &updates {
                    // Extract chat_id and text from message
                    if let Some(msg) = &update.message {
                        let chat_id = msg.chat.id.to_string();
                        let mut text = msg.text.clone().unwrap_or_default();
                        
                        // Update username if available
                        let username = msg.from.as_ref().map(|u| u.username.as_deref());
                        if let Some(uname) = username {
                            update_user_username(&chat_id, uname);
                        }
                        
                        // Check if bot is mentioned in group (group chats have negative IDs)
                        let chat_id_i64: i64 = chat_id.parse().unwrap_or(0);
                        let is_group = chat_id_i64 < 0;
                        let is_mention = if is_group && !text.starts_with('/') {
                            let mention = format!("@{}", info.username);
                            if text.to_lowercase().contains(&mention.to_lowercase()) {
                                // Remove mention from text
                                text = text.replace(&mention, "").replace(&mention.to_lowercase(), "").trim().to_string();
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        
                        // Skip if just mentioned without any actual text
                        if text.is_empty() {
                            continue;
                        }
                        
                        if !text.is_empty() {
                            // Check if this is the first message from this chat
                            let is_first = first_message.get(&chat_id).is_none();
                            if is_first {
                                first_message.insert(chat_id.clone(), true);
                            }
                            
                            // Process command or message
                            if text.starts_with(&commands.prefix()) || text.starts_with('/') {
                                // Check for /code command (coding agent)
                                let trimmed = text.trim_start_matches(&commands.prefix()).trim_start_matches('/');
                                if trimmed.starts_with("code") {
                                    // Check guest access first
                                    let chat_id_str = chat_id.to_string();
                                    let response = match can_use_privileged(&chat_id_str) {
                                        Ok(false) => {
                                            "❌ Access denied.\n\nUse /connect to request one-time guest access,\nor ask the owner to add you.".to_string()
                                        }
                                        Err(e) => format!("Error: {}", e),
                                        _ => {
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
                                        }
                                    };
                                    
                                    tracing::info!("Sending response to chat_id {}: {}", chat_id, &response[..response.len().min(100)]);
                                    if let Err(e) = bot.send_message(&chat_id, &response).await {
                                        tracing::error!("Failed to send message: {}", e);
                                    }
                                } else {
                                    let cmd_parts: Vec<&str> = trimmed.split_whitespace().collect();
                                    let cmd_name = cmd_parts.first().unwrap_or(&"").to_string();
                                    let args: Vec<String> = cmd_parts[1..].iter().map(|s| s.to_string()).collect();
                                    
                                    let msg = Message::from_command(&chat_id, cmd_name, args);
                                    let response = match commands.handle(&msg) {
                                        Ok(Some(response)) => response,
                                        Ok(None) => continue,
                                        Err(e) => format!("Error: {}", e),
                                    };
                                    
                                    tracing::info!("Sending response to chat_id {}: {}", chat_id, &response[..response.len().min(100)]);
                                    if let Err(e) = bot.send_message(&chat_id, &response).await {
                                        tracing::error!("Failed to send message: {}", e);
                                    }
                                }
                            } else {
                                // Auto-route: detect intent and route to appropriate handler
                                match route_message(&text, &chat_id, &mut conversations, &llm, &system_prompt).await {
                                    Some(resp) => {
                                        // Send response
                                        tracing::info!("Sending response to chat_id {}: {}", chat_id, &resp[..resp.len().min(100)]);
                                        if let Err(e) = bot.send_message(&chat_id, &resp).await {
                                            tracing::error!("Failed to send message: {}", e);
                                        }
                                    }
                                    None => {
                                        // Echo mode when LLM is not available
                                        let echo = format!("Echo: {}", text);
                                        if let Err(e) = bot.send_message(&chat_id, &echo).await {
                                            tracing::error!("Failed to send message: {}", e);
                                        }
                                    }
                                }
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

/// Connect command for guest access

/// Check if user can use privileged commands (/code, /kiro)
fn can_use_privileged(user_id: &str) -> Result<bool, String> {
    let config = Config::load("config.yaml").map_err(|e| e.to_string())?;
    
    // Whitelist users always have access
    if config.whitelist.enabled && config.whitelist.users.contains(&user_id.to_string()) {
        return Ok(true);
    }
    
    // Check if guest has been approved
    if config.guests.enabled && config.guests.approved.contains(&user_id.to_string()) {
        return Ok(true);
    }
    
    Ok(false)
}

/// Start command - shows welcome message
fn register_start_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    commands.register(Command::new("start")
        .with_description("Start conversation")
        .with_handler(|msg| {
            let chat_id = &msg.chat_id;
            let settings = get_user_settings(chat_id);
            let lang = settings.map(|s| s.language).unwrap_or_else(|| "en".to_string());
            Ok(generate_greeting("carik-bot", &lang))
        }));
}

fn register_connect_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    commands.register(Command::new("connect")
        .with_description("Request one-time access (guests)")
        .with_usage("/connect")
        .with_handler(|msg| {
            let Content::Command { name: _, args: _ } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            let user_id = msg.chat_id.clone();
            
            // Load config
            let config = Config::load("config.yaml").map_err(|e| 
                crate::application::errors::CommandError::ExecutionFailed(e.to_string()))?;
            
            // Check if whitelist is enabled - if so, connect not needed
            if config.whitelist.enabled && config.whitelist.users.contains(&user_id) {
                return Ok("You already have full access!".to_string());
            }
            
            // Check if already pending approval
            if config.guests.pending.contains(&user_id) {
                return Ok("⏳ Your request is pending approval.\n\nWait for the owner to approve you.".to_string());
            }
            
            // Check if already approved
            if config.guests.approved.contains(&user_id) {
                return Ok("✅ You're already approved! Use /code or /kiro".to_string());
            }
            
            // Add to pending list
            let mut guests = config.guests.clone();
            guests.pending.push(user_id.clone());
            
            let mut new_config = config.clone();
            new_config.guests = guests;
            new_config.save("config.yaml").map_err(|e| 
                crate::application::errors::CommandError::ExecutionFailed(e.to_string()))?;
            
            Ok("✅ Request sent! Your ID: {}\n\nWait for owner to approve with /approve {}".to_string())
        }));
}

/// Approve command for owner to approve guest requests
fn register_approve_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    commands.register(Command::new("approve")
        .with_description("Approve guest request (owner only)")
        .with_usage("/approve <user_id>")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            let user_id = msg.chat_id.clone();
            
            // Check if owner (only allow owner to approve)
            let config = Config::load("config.yaml").map_err(|e| 
                crate::application::errors::CommandError::ExecutionFailed(e.to_string()))?;
            
            if !config.whitelist.users.contains(&user_id) {
                return Ok("❌ Only owner can approve requests.".to_string());
            }
            
            if args.is_empty() {
                // List pending requests
                let pending = config.guests.pending;
                if pending.is_empty() {
                    return Ok("No pending requests.".to_string());
                }
                return Ok(format!("Pending requests:\n{}\n\nUse /approve <id> to approve.", 
                    pending.iter().map(|id| format!("- {}", id)).collect::<Vec<_>>().join("\n")));
            }
            
            let target_id = args[0].clone();
            
            // Check if in pending
            if !config.guests.pending.contains(&target_id) {
                return Ok("User not in pending list.".to_string());
            }
            
            // Move from pending to approved
            let mut guests = config.guests.clone();
            guests.pending.retain(|id| id != &target_id);
            guests.approved.push(target_id.clone());
            
            // Also add to whitelist
            let mut whitelist = config.whitelist.clone();
            if !whitelist.users.contains(&target_id) {
                whitelist.users.push(target_id.clone());
            }
            
            let mut new_config = config;
            new_config.guests = guests;
            new_config.whitelist = whitelist;
            new_config.save("config.yaml").map_err(|e| 
                crate::application::errors::CommandError::ExecutionFailed(e.to_string()))?;
            
            // Get user's language preference
            let settings = get_user_settings(&target_id);
            let lang = settings.map(|s| s.language).unwrap_or_else(|| "en".to_string());
            let greeting = generate_greeting("carik-bot", &lang);
            Ok(format!("✅ Approved! User: {}\n\n{}\n\nThey can now use /code or /kiro", target_id, greeting))
        }));
}

/// Workspace management
/// Get platform-specific carik-bot home directory
fn get_carik_home() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("APPDATA").map(|p| format!("{}\\carik-bot", p)).unwrap_or_else(|_| ".carik-bot".to_string())
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME").map(|p| format!("{}/.carik-bot", p)).unwrap_or_else(|_| ".carik-bot".to_string())
    } else {
        // Linux and others
        std::env::var("HOME").map(|p| format!("{}/.carik-bot", p)).unwrap_or_else(|_| "/home/ubuntu/.carik-bot".to_string())
    }
}

/// Get config file path
fn get_config_path() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("APPDATA").map(|p| format!("{}\\carik-bot\\config.yaml", p)).unwrap_or_else(|_| "config.yaml".to_string())
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME").map(|p| format!("{}/Library/Application Support/carik-bot/config.yaml", p)).unwrap_or_else(|_| "config.yaml".to_string())
    } else {
        // Linux
        std::env::var("XDG_CONFIG_HOME").map(|p| format!("{}/carik-bot/config.yaml", p))
            .or_else(|_| std::env::var("HOME").map(|p| format!("{}/.config/carik-bot/config.yaml", p)))
            .unwrap_or_else(|_| "config.yaml".to_string())
    }
}

// For backward compatibility - use environment variable or default
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

/// Detect if user message is a coding task
fn is_coding_intent(text: &str) -> bool {
    let coding_keywords = [
        "code", "write", "create", "build", "make", "debug", "fix", 
        "program", "script", "function", "class", "algorithm",
        "implement", "develop", "refactor", "optimize",
        "python", "javascript", "rust", "java", "golang", "typescript",
        "app", "application", "website", "web", "api", "database",
    ];
    
    let lower = text.to_lowercase();
    coding_keywords.iter().any(|kw| lower.contains(kw))
}

/// Detect if user message is asking for news/RSS
fn is_rss_intent(text: &str) -> bool {
    let rss_keywords = [
        "news", "headlines", "latest", "feed", "rss",
        "berita", "headline", "update", "breaking",
    ];
    
    let lower = text.to_lowercase();
    rss_keywords.iter().any(|kw| lower.contains(kw))
}

/// Detect if user message is asking for a skill/tool
fn detect_skill(text: &str) -> Option<String> {
    let skill_keywords = [
        ("weather", &["weather", "cuaca"]),
        ("github", &["github", "git"]),
        ("spotify", &["spotify", "music"]),
        ("weather", &["weather", "forecast"]),
        ("obsidian", &["obsidian", "note"]),
    ];
    
    let lower = text.to_lowercase();
    for (skill, keywords) in skill_keywords {
        if keywords.iter().any(|kw| lower.contains(kw)) {
            return Some(skill.to_string());
        }
    }
    None
}

/// Detect if user is responding to news selection and extract source
fn detect_news_source(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    
    // Map keywords to RSS URLs
    let sources = [
        (vec!["yahoo", "yahoo news"], "https://news.yahoo.com/rss/topstories"),
        (vec!["google", "google news"], "https://news.google.com/rss"),
        (vec!["bbc", "bbc news"], "http://feeds.bbci.co.uk/news/rss.xml"),
        (vec!["techcrunch"], "https://techcrunch.com/feed/"),
        (vec!["hn", "hacker news", "hackernews"], "https://hnrss.org/newest"),
        (vec!["cna", "channel newsasia", "channelnewsasia"], "https://www.channelnewsasia.com/rss"),
        (vec!["reuters"], "https://www.reutersagency.com/feed/"),
        (vec!["bbc world"], "http://feeds.bbci.co.uk/news/world/rss.xml"),
    ];
    
    for (keywords, _url) in sources {
        for kw in keywords {
            if lower.contains(kw) {
                return Some(kw.to_string());
            }
        }
    }
    
    None
}

/// Detect topic/country from news query and return (topic_name, rss_url)
fn detect_news_topic(text: &str) -> Option<(String, String)> {
    let lower = text.to_lowercase();
    
    // G20 Countries RSS feeds
    let topic_sources = [
        // G20 Countries
        (vec!["argentina", "argentine"], "Argentina", "https://www.batimes.com.ar/feed"),
        (vec!["australia", "australian"], "Australia", "https://www.abc.net.au/news/feed/45910/rss.xml"),
        (vec!["brazil", "brazilian"], "Brazil", "https://agenciabrasil.ebc.com.br/rss/ultimasnoticias/feed.xml"),
        (vec!["canada", "canadian"], "Canada", "https://www.cbc.ca/cmlink/rss-topstories"),
        (vec!["china", "chinese"], "China", "https://www.scmp.com/rss/91/feed"),
        (vec!["france", "french"], "France", "https://www.france24.com/en/rss"),
        (vec!["germany", "german", "deutsch"], "Germany", "https://rss.dw.com/rdf/rss-en-all"),
        (vec!["india", "indian"], "India", "https://timesofindia.indiatimes.com/rssfeedstopstories.cms"),
        (vec!["indonesia", "indonesian"], "Indonesia", "https://en.antaranews.com/rss/news.xml"),
        (vec!["italy", "italian"], "Italy", "https://www.agi.it/rss"),
        (vec!["japan", "japanese"], "Japan", "https://www.japantimes.co.jp/feed"),
        (vec!["mexico", "mexican"], "Mexico", "https://www.eluniversal.com.mx/rss.xml"),
        (vec!["russia", "russian"], "Russia", "https://www.rt.com/rss/news/"),
        (vec!["saudi", "saudi arabia"], "Saudi Arabia", "https://www.arabnews.com/rss.xml"),
        (vec!["south africa"], "South Africa", "https://mg.co.za/feed/"),
        (vec!["korea", "south korean", "korean"], "South Korea", "https://en.yna.co.kr/RSS/news.xml"),
        (vec!["turkey", "turkish"], "Turkey", "https://www.hurriyet.com.tr/rss/anasayfa"),
        (vec!["uk", "britain", "british", "england"], "UK", "http://feeds.bbci.co.uk/news/world/rss.xml"),
        (vec!["us", "usa", "america", "american", "united states"], "USA", "https://rss.nytimes.com/services/xml/rss/nyt/HomePage.xml"),
        (vec!["eu", "europe", "european"], "EU", "https://www.euronews.com/rss?level=vertical&name=news"),
        
        // Southeast Asia
        (vec!["singapore", "singaporean"], "Singapore", "https://www.straitstimes.com/news/world/rss.xml"),
        (vec!["malaysia", "malaysian"], "Malaysia", "https://www.thestar.com.my/rss/news/nation"),
        (vec!["vietnam", "vietnamese"], "Vietnam", "https://vietnamnews.vn/rss/politics.rss"),
        (vec!["thailand", "thai"], "Thailand", "https://www.bangkokpost.com/rss/data/topstories.xml"),
        (vec!["philippines", "philippine", "filipino"], "Philippines", "https://www.inquirer.net/fullfeed"),
        (vec!["myanmar", "burmese"], "Myanmar", "https://www.irrawaddy.com/feed"),
        (vec!["cambodia", "cambodian"], "Cambodia", "https://www.khmertimeskh.com/feed/"),
        
        // South & Central Asia
        (vec!["pakistan", "pakistani"], "Pakistan", "https://www.dawn.com/feeds/home/"),
        (vec!["bangladesh", "bangladeshi"], "Bangladesh", "https://www.thedailystar.net/rss.xml"),
        (vec!["sri lanka", "sri lankan"], "Sri Lanka", "https://www.dailymirror.lk/rss/news"),
        (vec!["nepal", "nepalese"], "Nepal", "https://kathmandupost.com/rss"),
        (vec!["kazakhstan", "kazakh"], "Kazakhstan", "https://www.inform.kz/en/rss"),
        (vec!["uzbekistan", "uzbek"], "Uzbekistan", "https://uzreport.news/rss"),
        
        // East Asia & Middle East (Non-G20)
        (vec!["taiwan", "taiwanese"], "Taiwan", "https://focustaiwan.tw/rss/get-rss.aspx?type=aall"),
        (vec!["israel", "israeli"], "Israel", "https://www.haaretz.com/cmlink/1.4621118"),
        (vec!["uae", "united arab emirates", "dubai"], "UAE", "https://gulfnews.com/rss"),
        (vec!["qatar", "qatari"], "Qatar", "https://www.aljazeera.com/xml/rss/all.xml"),
        (vec!["iran", "iranian"], "Iran", "https://en.mehrnews.com/rss"),
        (vec!["mongolia", "mongolian"], "Mongolia", "https://www.montsame.mn/en/rss"),
        (vec!["north korea", "north korean"], "North Korea", "https://www.kcna.watch/feed/"),
        
        // Topics
        (vec!["technology", "tech"], "Technology", "https://techcrunch.com/feed/"),
        (vec!["business", "finance", "economy"], "Business", "https://news.yahoo.com/rss/business"),
        (vec!["sports", "sport"], "Sports", "https://news.yahoo.com/rss/sports"),
        (vec!["entertainment", "movie", "film"], "Entertainment", "https://news.yahoo.com/rss/entertainment"),
    ];
    
    for (keywords, display, url) in topic_sources {
        for kw in keywords {
            if lower.contains(kw) {
                return Some((display.to_string(), url.to_string()));
            }
        }
    }
    
    None
}

/// Check if conversation is about news (has RSS context)
fn is_news_conversation(conversation: &[LLMMessage]) -> bool {
    for msg in conversation {
        if msg.role == "assistant" && msg.content.contains("news source") || 
           msg.content.contains("RSS feed") || msg.content.contains("Which source") {
            return true;
        }
    }
    false
}

/// Route message to appropriate handler with conversation history
async fn route_message(
    text: &str, 
    chat_id: &str, 
    conversations: &mut std::collections::HashMap<String, Vec<LLMMessage>>, 
    llm: &Option<GroqProvider>, 
    system_prompt: &str
) -> Option<String> {
    // Get user settings
    let user_settings = get_user_settings(chat_id);
    
    // Debug: log the language
    if let Some(ref settings) = user_settings {
        tracing::info!("User {} language: {}", chat_id, settings.language);
    }
    
    // Build personalized system prompt with language instruction
    let mut final_prompt = system_prompt.to_string();
    
    // Add language instruction based on user preference
    if let Some(settings) = &user_settings {
        let lang_instruction = match settings.language.as_str() {
            "jv" => "\n\nIMPORTANT: Respond in Javanese language (ꦧꦱꦗꦮ).",
            "id" => "\n\nIMPORTANT: Respond in Indonesian language.",
            _ => "",
        };
        final_prompt = format!("{}{}", final_prompt, lang_instruction);
        
        // Add custom prompt if any
        if let Some(custom_prompt) = &settings.system_prompt {
            final_prompt = format!("{}\n\nPersonal instructions: {}", final_prompt, custom_prompt);
        }
    }
    
    // Get or create conversation history for this chat
    let conversation = conversations.entry(chat_id.to_string()).or_insert_with(Vec::new);
    
    // Keep conversation history limited (last 10 messages)
    if conversation.len() > 20 {
        conversation.remove(0);
        conversation.remove(0);
    }
    
    // Check if user is responding to news selection
    if is_news_conversation(conversation) {
        if let Some(source) = detect_news_source(text) {
            tracing::info!("Detected news source selection: {}, fetching RSS", source);
            
            // Map source to URL
            let url = match source.as_str() {
                "yahoo" | "yahoo news" => "https://news.yahoo.com/rss/topstories",
                "google" | "google news" => "https://news.google.com/rss",
                "bbc" | "bbc news" => "http://feeds.bbci.co.uk/news/rss.xml",
                "bbc world" => "http://feeds.bbci.co.uk/news/world/rss.xml",
                "techcrunch" => "https://techcrunch.com/feed/",
                "hn" | "hacker news" | "hackernews" => "https://hnrss.org/newest",
                "cna" | "channel newsasia" | "channelnewsasia" => "https://www.channelnewsasia.com/rss",
                "reuters" => "https://www.reutersagency.com/feed/",
                _ => "https://news.yahoo.com/rss/topstories",
            };
            
            // Fetch RSS and use LLM to summarize
            let rss_content = fetch_rss_feed_blocking(url);
            
            // Format response with source name
            let source_display = source.replace("yahoo news", "Yahoo News")
                .replace("google news", "Google News")
                .replace("bbc news", "BBC News")
                .replace("channel newsasia", "Channel NewsAsia")
                .replace("channelnewsasia", "Channel NewsAsia")
                .replace("hacker news", "Hacker News")
                .replace("hackernews", "Hacker News");
            
            let header = format!("📰 *{} News*\n\n", source_display);
            
            // Use LLM to summarize
            if let Some(ref llm) = llm {
                let summarize_prompt = format!(
                    "You're a news reporter. Summarize these headlines in a friendly, conversational way (2-3 sentences, then bullet points). MUST include the article URL after each headline like this: Headline Title\n🔗 https://example.com\n\n{}",
                    rss_content
                );
                
                let messages = vec![
                    LLMMessage::system(&final_prompt),
                    LLMMessage::user(&summarize_prompt),
                ];
                
                match llm.chat(messages, None, Some(0.7), None).await {
                    Ok(response) => {
                        let final_response = format!("{}{}", header, response.content);
                        conversation.push(LLMMessage::user(text.to_string()));
                        conversation.push(LLMMessage::assistant(final_response.clone()));
                        return Some(final_response);
                    }
                    Err(_) => {
                        // Fallback to raw
                    }
                }
            }
            
            // Fallback to raw feed
            let response = format!("📰 *{}*\n\n{}", source_display, rss_content);
            conversation.push(LLMMessage::user(text.to_string()));
            conversation.push(LLMMessage::assistant(response.clone()));
            return Some(response);
        }
    }
    
    // Check for RSS/news intent - fetch and summarize
    if is_rss_intent(text) {
        tracing::info!("Detected RSS intent, fetching and summarizing news");
        
        // Detect topic (e.g., India, Indonesia, technology) - includes specific RSS URL
        let topic = detect_news_topic(text);
        
        // Check if user specified a source explicitly
        let explicit_source = detect_news_source(text);
        
        // Determine URL: topic-specific > explicit source > default
        let (url, source_display) = if let Some((topic_name, topic_url)) = &topic {
            // Use topic's RSS source
            (topic_url.clone(), topic_name.clone())
        } else if let Some(source) = explicit_source {
            let url = match source.as_str() {
                "yahoo" | "yahoo news" => "https://news.yahoo.com/rss/topstories",
                "google" | "google news" => "https://news.google.com/rss",
                "bbc" | "bbc news" => "http://feeds.bbci.co.uk/news/rss.xml",
                "bbc world" => "http://feeds.bbci.co.uk/news/world/rss.xml",
                "techcrunch" => "https://techcrunch.com/feed/",
                "hn" | "hacker news" | "hackernews" => "https://hnrss.org/newest",
                "cna" | "channel newsasia" | "channelnewsasia" => "https://www.channelnewsasia.com/rss",
                "reuters" => "https://www.reutersagency.com/feed/",
                _ => "https://news.yahoo.com/rss/topstories",
            };
            let display = source.replace("yahoo news", "Yahoo News")
                .replace("google news", "Google News")
                .replace("bbc news", "BBC News")
                .replace("channel newsasia", "Channel NewsAsia")
                .replace("channelnewsasia", "Channel NewsAsia")
                .replace("hacker news", "Hacker News")
                .replace("hackernews", "Hacker News");
            (url.to_string(), display)
        } else {
            ("https://news.yahoo.com/rss/topstories".to_string(), "Yahoo News".to_string())
        };
        
        // Fetch RSS content
        let rss_content = fetch_rss_feed_blocking(&url);
        
        // Build header with topic if detected (needed for error case too)
        let header = if let Some((topic_name, _)) = &topic {
            format!("📰 *Latest {} News*\n\n", topic_name)
        } else {
            format!("📰 *Latest {} News*\n\n", source_display)
        };
        
        // Check if RSS fetch failed - if so, return error directly without LLM
        if rss_content.starts_with("❌") {
            let error_response = format!("{}Sorry, couldn't fetch the news feed. Please try again or try a different source like /rss google\n\nError: {}", header, rss_content);
            conversation.push(LLMMessage::user(text.to_string()));
            conversation.push(LLMMessage::assistant(error_response.clone()));
            return Some(error_response);
        }
        
        // Use LLM to summarize
        if let Some(ref llm) = llm {
            let topic_hint = topic.as_ref().map(|(t, _)| format!(" Focus on {} news.", t)).unwrap_or_default();
            let summarize_prompt = format!(
                "You're a news reporter. Summarize these headlines in a friendly, conversational way (2-3 sentences max, then bullet points). MUST include the article URL after each headline like this: Headline Title\n🔗 https://example.com{}\n\nHeadlines:\n{}",
                topic_hint, rss_content
            );
            
            let messages = vec![
                LLMMessage::system(&final_prompt),
                LLMMessage::user(&summarize_prompt),
            ];
            
            match llm.chat(messages, None, Some(0.7), None).await {
                Ok(response) => {
                    let final_response = format!("{}{}", header, response.content);
                    conversation.push(LLMMessage::user(text.to_string()));
                    conversation.push(LLMMessage::assistant(final_response.clone()));
                    return Some(final_response);
                }
                Err(e) => {
                    // Fallback to raw feed on error
                    let fallback = format!("{}{}", header, rss_content);
                    return Some(fallback);
                }
            }
        }
        
        // No LLM - return raw feed
        let response = format!("{}{}", header, rss_content);
        conversation.push(LLMMessage::user(text.to_string()));
        conversation.push(LLMMessage::assistant(response.clone()));
        return Some(response);
    }
    
    // Check for coding intent
    if is_coding_intent(text) {
        tracing::info!("Detected coding intent, routing to Kiro");
        let response = execute_kiro_cli(text).await;
        return Some(response);
    }
    
    // Check for skill intent
    if let Some(skill) = detect_skill(text) {
        tracing::info!("Detected skill: {}", skill);
        // TODO: Load skill.md and execute
        return Some(format!("Skill '{}' detected. Skill execution coming soon!", skill));
    }
    
    // Route to LLM with conversation history
    if let Some(ref llm) = llm {
        tracing::info!("Routing to LLM with history");
        
        // Build messages with history
        let mut messages = conversation.clone();
        messages.push(LLMMessage::system(&final_prompt));
        messages.push(LLMMessage::user(text));
        
        match llm.chat(messages.clone(), None, Some(0.7), None).await {
            Ok(response) => {
                // Add to conversation history
                conversation.push(LLMMessage::user(text.to_string()));
                conversation.push(LLMMessage::assistant(response.content.clone()));
                return Some(response.content);
            }
            Err(e) => return Some(format!("LLM Error: {}", e)),
        }
    }
    
    None
}

/// Kiro CLI tmux session management
const KIRO_SOCKET: &str = "/tmp/carik-kiro.sock";
const KIRO_SESSION: &str = "carik-kiro";

fn register_kiro_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    // Main kiro command - handles /kiro <prompt>
    commands.register(Command::new("kiro")
        .with_description("Run kiro-cli in Docker")
        .with_usage("/kiro <prompt>")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            // Check access
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => {
                    return Ok("❌ Access denied.\n\nUse /connect to request one-time guest access,\nor ask the owner to add you.".to_string());
                }
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            
            if args.is_empty() {
                return Ok("Usage: /kiro <prompt>\n\nSubcommands:\n/kiro-status - Check if running\n/kiro-log - See output\n/kiro-kill - Stop session\n/kiro-new - Start new container\n/kiro-fresh - Start fresh conversation\n/kiro-ls - List workspace files\n/kiro-read <file> - Read file\n/kiro-write <file> <content> - Write file\n\nNote: /kiro automatically resumes last conversation.".to_string());
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
        .with_handler(|msg| {
            // Check access
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => {
                    return Ok("❌ Access denied. Use /connect first.".to_string());
                }
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            kiro_log().map_err(|e| crate::application::errors::CommandError::ExecutionFailed(e))
        }));
    
    commands.register(Command::new("kiro-kill")
        .with_description("Kill kiro session")
        .with_handler(|msg| {
            // Check access
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => {
                    return Ok("❌ Access denied. Use /connect first.".to_string());
                }
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            kiro_kill().map_err(|e| crate::application::errors::CommandError::ExecutionFailed(e))
        }));
    
    // kiro new - start fresh conversation
    commands.register(Command::new("kiro-new")
        .with_description("Start new kiro conversation")
        .with_handler(|msg| {
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => return Ok("❌ Access denied. Use /connect first.".to_string()),
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            // Kill existing and start new
            let _ = std::process::Command::new("docker")
                .args(["kill", "kiro-persistent"])
                .output();
            let _ = std::process::Command::new("docker")
                .args(["rm", "kiro-persistent"])
                .output();
            
            // Recreate container - inherit env from host
            let cmd = r#"docker run -d --name kiro-persistent \
                -v /home/ubuntu/.kiro:/root/.kiro \
                -v /home/ubuntu/.local/share/kiro-cli:/root/.local/share/kiro-cli \
                -v /home/ubuntu/.carik-bot:/workspace \
                -v /home/ubuntu/.local/bin/kiro-cli:/usr/local/bin/kiro-cli \
                -v /home/ubuntu/.aws:/root/.aws \
                -e PATH="/root/.local/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
                --env-file /home/ubuntu/.aws/credentials \
                --workdir /workspace \
                ubuntu:latest sleep infinity"#;
            
            let output = std::process::Command::new("bash")
                .args(["-c", cmd])
                .output();
            
            let success = output.as_ref().map(|o| o.status.success()).unwrap_or(false);
            if success {
                Ok("🔄 Started new Kiro session!".to_string())
            } else {
                Ok("❌ Failed to start new session.".to_string())
            }
        }));
    
    // kiro ls - list workspace files
    commands.register(Command::new("kiro-ls")
        .with_description("List workspace files")
        .with_handler(|msg| {
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => return Ok("❌ Access denied. Use /connect first.".to_string()),
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            
            let output = std::process::Command::new("docker")
                .args(["exec", "kiro-persistent", "ls", "-la", "/workspace/default-workspace"])
                .output();
            
            match output {
                Ok(o) if o.status.success() => {
                    let files = String::from_utf8_lossy(&o.stdout);
                    Ok(format!("📁 Workspace files:\n```\n{}```", files))
                }
                _ => Ok("❌ Kiro not running. Use /kiro first.".to_string())
            }
        }));
    
    // kiro read - read file from workspace
    commands.register(Command::new("kiro-read")
        .with_description("Read file from workspace")
        .with_usage("/kiro-read <filename>")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => return Ok("❌ Access denied. Use /connect first.".to_string()),
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            
            if args.is_empty() {
                return Ok("Usage: /kiro-read <filename>".to_string());
            }
            
            let filename = args.join(" ");
            let filepath = format!("/workspace/default-workspace/{}", filename);
            
            let output = std::process::Command::new("docker")
                .args(["exec", "kiro-persistent", "cat", &filepath])
                .output();
            
            match output {
                Ok(o) if o.status.success() => {
                    let content = String::from_utf8_lossy(&o.stdout);
                    if content.len() > 2000 {
                        Ok(format!("📄 {} (truncated):\n```\n{}...```", filename, &content[..2000]))
                    } else {
                        Ok(format!("📄 {}:\n```\n{}```", filename, content))
                    }
                }
                _ => Ok(format!("❌ File not found: {}", filename))
            }
        }));
    
    // kiro write - write file to workspace
    commands.register(Command::new("kiro-write")
        .with_description("Write file to workspace")
        .with_usage("/kiro-write <filename> <content>")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => return Ok("❌ Access denied. Use /connect first.".to_string()),
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            
            if args.len() < 2 {
                return Ok("Usage: /kiro-write <filename> <content>\n\nTip: Use quotes for content with spaces.".to_string());
            }
            
            let filename = &args[0];
            let content = args[1..].join(" ");
            let filepath = format!("/workspace/default-workspace/{}", filename);
            
            // Write using docker exec with bash
            let cmd = format!("echo '{}' > {}", 
                content.replace("'", "'\\''"),
                filepath
            );
            
            let output = std::process::Command::new("docker")
                .args(["exec", "kiro-persistent", "bash", "-c", &cmd])
                .output();
            
            match output {
                Ok(o) if o.status.success() => Ok(format!("✅ Wrote to: {}", filename)),
                _ => Ok("❌ Failed to write file.".to_string())
            }
        }));
    
    // kiro fresh - start new conversation (no resume)
    commands.register(Command::new("kiro-fresh")
        .with_description("Start fresh conversation")
        .with_handler(|msg| {
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => return Ok("❌ Access denied. Use /connect first.".to_string()),
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            
            let workspace_dir = get_docker_workspace_dir();
            let prompt = "Start a fresh conversation.";
            
            let cmd = format!(
                "cd {} && kiro-cli chat --no-interactive --trust-all-tools \"{}\"",
                workspace_dir,
                prompt.replace("\"", "\\\"")
            );
            
            let output = std::process::Command::new("docker")
                .args(["exec", KIRO_CONTAINER, "bash", "-c", &cmd])
                .output();
            
            let output = match output {
                Ok(o) => o,
                Err(e) => return Ok(format!("❌ Error: {}", e)),
            };
            
            if output.status.success() {
                let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
                Ok(format!("🔄 Fresh conversation started!\n\n{}", &stdout[..stdout.len().min(500)]))
            } else {
                let err = String::from_utf8_lossy(&output.stderr);
                Ok(format!("❌ Error: {}", err))
            }
        }));
    
    // Groq model - switch Groq LLM model
    commands.register(Command::new("model")
        .with_description("Switch Groq LLM model")
        .with_usage("/model [llama33|llama4|kimi|qwen|gpt-oss]")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => return Ok("❌ Access denied. Use /connect first.".to_string()),
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            
            if args.is_empty() {
                return Ok("Available Groq models:\n• llama33 - Llama 3.3 70B\n• llama4 - Llama 4 Scout\n• kimi - Kimi Audio\n• qwen - Qwen 2.5 72B\n• gpt-oss - GPT-4o-mini\n\nCurrent model: llama33\nUsage: /model llama33".to_string());
            }

            let model = args[0].to_lowercase();
            // Map to actual Groq model names
            let model_name = match model.as_str() {
                "llama33" => "llama-3.3-70b-versatile",
                "llama4" => "llama-4-scout-17b-16e",
                "kimi" => "kimi-audio-1.5-preview",
                "qwen" => "qwen-2.5-72b-instruct",
                "gpt-oss" => "gpt-4o-mini",
                _ => return Ok("Unknown model. Use: llama33, llama4, kimi, qwen, gpt-oss".to_string()),
            };
            
            // Save model preference to file
            let _ = std::fs::write("/home/ubuntu/.carik-bot/groq-model.txt", model_name);

            Ok(format!("✅ Groq model set to: {}\n\nNote: Restart bot for changes to take effect.", args[0]))
        }));
    
    // kiro model - switch Kiro model
    commands.register(Command::new("kiro-model")
        .with_description("Switch Kiro model")
        .with_usage("/kiro-model [auto|opus|sonnet|haiku]")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            match can_use_privileged(&msg.chat_id) {
                Ok(false) => return Ok("❌ Access denied. Use /connect first.".to_string()),
                Err(e) => return Ok(format!("Error: {}", e)),
                _ => {}
            }
            
            if args.is_empty() {
                return Ok("Available models:\n• auto - Auto-select (default)\n• opus - Claude Opus 4.6\n• sonnet - Claude Sonnet 4.6\n• haiku - Claude Haiku 4.5\n\nUsage: /kiro-model opus".to_string());
            }
            
            let model = args[0].to_lowercase();
            // Map to actual Claude model names
            let model_arg = match model.as_str() {
                "opus" => "--model claude-opus-4.6",
                "sonnet" => "--model claude-sonnet-4.6",
                "haiku" => "--model claude-haiku-4.5",
                "auto" => "",
                _ => return Ok("Unknown model. Use: auto, opus, sonnet, or haiku".to_string()),
            };
            
            // Save model preference to file
            let _ = std::fs::write("/home/ubuntu/.carik-bot/kiro-model.txt", &model_arg);
            
            Ok(format!("✅ Model set to: {}\n\nNote: This will be used for next /kiro command.", model))
        }));
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
        
        // Check for model preference
        let model_arg = std::fs::read_to_string("/home/ubuntu/.carik-bot/kiro-model.txt")
            .map(|m| m.trim().to_string())
            .unwrap_or_default();
        
        // Build command - quote the prompt to handle spaces
        let kiro_args = if model_arg.is_empty() {
            "--no-interactive --trust-all-tools --resume".to_string()
        } else {
            format!("{} --no-interactive --trust-all-tools --resume", model_arg)
        };
        
        // Quote the prompt to handle multi-word prompts
        let cmd = format!(
            "cd {} && kiro-cli chat {} \"{}\"",
            workspace_dir,
            kiro_args,
            prompt.replace("\"", "\\\"")
        );
        
        let output = std::process::Command::new("docker")
        
            .args(["exec", KIRO_CONTAINER, "bash", "-c", &cmd])
            .output()
            .map_err(|e| e.to_string())?;
        
        if output.status.success() {
            let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
            let cleaned = clean_kiro_output(&stdout);
            tracing::info!("Kiro stdout length: {}", cleaned.len());
            // Save output for kiro-log
            match std::fs::write("/home/ubuntu/.carik-bot/kiro-last-output.txt", cleaned.to_string()) {
                Ok(_) => tracing::info!("Output saved to file"),
                Err(e) => tracing::error!("Failed to save output: {}", e),
            }
            return Ok(format!("📤 Kiro response:\n{}\n\nUse /kiro-log for full output.", &cleaned[..cleaned.len().min(500)]));
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Kiro error: {}", err);
            let _ = std::fs::write("/home/ubuntu/.carik-bot/kiro-last-output.txt", err.to_string());
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
    // Try to read from stored output file first (use workspace path)
    let output_file = "/home/ubuntu/.carik-bot/kiro-last-output.txt";
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

/// Clean up Kiro output - remove credits, time, and trailing artifacts
fn clean_kiro_output(s: &str) -> String {
    let mut output = s.to_string();
    
    // Remove the credits/time footer line (e.g., "▸ Credits: 0.03 • Time: 2s")
    if let Some(pos) = output.rfind("Credits:") {
        output = output[..pos].trim().to_string();
    }
    
    // Remove ANSI escape sequences
    output = strip_ansi(&output);
    
    // Remove any remaining trailing whitespace
    output.trim().to_string()
}

/// Rate limiting constants
const RATE_LIMIT_PER_MINUTE: i64 = 1;
const RATE_LIMIT_PER_HOUR: i64 = 20;

/// Get owner ID from environment variable
fn get_owner_id() -> Option<String> {
    std::env::var("BOT_OWNER_ID").ok()
}

/// Check if user is owner (from env var)
fn is_owner(user_id: &str) -> bool {
    if let Some(owner_id) = get_owner_id() {
        return owner_id == user_id;
    }
    // Fallback to config
    if let Ok(config) = Config::load("config.yaml") {
        return config.whitelist.enabled && config.whitelist.users.contains(&user_id.to_string());
    }
    false
}

/// Check if user is allowed based on role
fn get_user_role(user_id: &str) -> String {
    // First check if owner (from env var) - highest priority
    if is_owner(user_id) {
        return "owner".to_string();
    }
    
    // Then check database
    if let Ok(db_guard) = DB.lock() {
        if let Some(db) = db_guard.as_ref() {
            if let Ok(Some(user)) = db.get_user_by_telegram_id(user_id) {
                return user.role;
            }
        }
    }
    
    "guest".to_string()
}

/// Update user username when they send a message
fn update_user_username(user_id: &str, username: Option<&str>) {
    if let Some(username) = username {
        if let Ok(db_guard) = DB.lock() {
            if let Some(db) = db_guard.as_ref() {
                // Check if user exists
                if let Ok(Some(_user)) = db.get_user_by_telegram_id(user_id) {
                    // For now, we just ensure user exists
                    // Username updates would require adding a method to update username
                    let _ = db.add_user(user_id, Some(username), "user");
                }
            }
        }
    }
}

/// Get user settings from database
fn get_user_settings(user_id: &str) -> Option<database::UserSettings> {
    if let Ok(db_guard) = DB.lock() {
        if let Some(db) = db_guard.as_ref() {
            tracing::debug!("Getting settings for user: {}", user_id);
            if let Ok(settings) = db.get_user_settings(user_id) {
                tracing::debug!("Settings found: {:?}", settings);
                return settings;
            }
        }
    }
    None
}

/// Check rate limit for user
fn check_rate_limit(user_id: &str) -> Result<bool, String> {
    // Skip rate limiting for owner
    if get_user_role(user_id) == "owner" {
        return Ok(true);
    }
    
    // Get database
    let db_guard = DB.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    
    // Get user from database
    let user = db.get_user_by_telegram_id(user_id)
        .map_err(|e| e.to_string())?
        .ok_or("User not found")?;
    
    // Check minute rate limit
    let min_count = db.count_recent_queries(user.id, "query", 1)
        .map_err(|e| e.to_string())?;
    if min_count >= RATE_LIMIT_PER_MINUTE {
        return Ok(false);
    }
    
    // Check hourly rate limit
    let hour_count = db.count_hourly_queries(user.id, "query")
        .map_err(|e| e.to_string())?;
    if hour_count >= RATE_LIMIT_PER_HOUR {
        return Ok(false);
    }
    
    // Record the query
    db.record_query(user.id, "query").map_err(|e| e.to_string())?;
    
    Ok(true)
}

/// Register /users command for user management
fn register_users_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    commands.register(Command::new("users")
        .with_description("Manage users (owner/admin)")
        .with_usage("/users <list|add|remove> [args]")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            // Check if owner
            let role = get_user_role(&msg.chat_id);
            if role != "owner" && role != "admin" {
                return Ok("❌ Only owner/admin can manage users.".to_string());
            }
            
            let args_str = args.join(" ");
            let parts: Vec<&str> = args_str.split_whitespace().collect();
            
            match parts.first().map(|s| *s) {
                Some("list") | Some("ls") | None => {
                    // List all users
                    let db_guard = DB.lock().unwrap();
                    if let Some(db) = db_guard.as_ref() {
                        match db.list_users() {
                            Ok(users) => {
                                let mut response = "📋 *Users List*\n\n".to_string();
                                for user in users.iter().take(20) {
                                    response.push_str(&format!(
                                        "• {} (@{}) - {}\n",
                                        user.telegram_id,
                                        user.username.as_deref().unwrap_or("none"),
                                        user.role
                                    ));
                                }
                                if users.len() > 20 {
                                    response.push_str(&format!("\n... and {} more", users.len() - 20));
                                }
                                Ok(response)
                            }
                            Err(e) => Ok(format!("Error: {}", e))
                        }
                    } else {
                        Ok("Database not initialized".to_string())
                    }
                }
                Some("add") => {
                    if parts.len() < 3 {
                        return Ok("Usage: /users add <telegram_id> <role>\nRoles: owner, admin, user, guest".to_string());
                    }
                    let target_id = parts[1];
                    let role = parts[2];
                    
                    // Prevent adding owner role - owner can only be set via BOT_OWNER_ID env var
                    if role == "owner" {
                        return Ok("❌ Cannot add owner role. Owner is set via BOT_OWNER_ID environment variable.".to_string());
                    }
                    
                    // Check if target is owner (from env)
                    if is_owner(target_id) {
                        return Ok("❌ Cannot modify owner (set via BOT_OWNER_ID env var).".to_string());
                    }
                    
                    let db_guard = DB.lock().unwrap();
                    if let Some(db) = db_guard.as_ref() {
                        match db.add_user(target_id, None, role) {
                            Ok(_) => Ok(format!("✅ User {} added as {}", target_id, role)),
                            Err(e) => Ok(format!("Error adding user: {}", e))
                        }
                    } else {
                        Ok("Database not initialized".to_string())
                    }
                }
                Some("remove") => {
                    if parts.len() < 2 {
                        return Ok("Usage: /users remove <telegram_id>".to_string());
                    }
                    let target_id = parts[1];
                    
                    // Check if target is owner (from env)
                    if is_owner(target_id) {
                        return Ok("❌ Cannot remove owner (set via BOT_OWNER_ID env var).".to_string());
                    }
                    
                    let db_guard = DB.lock().unwrap();
                    if let Some(db) = db_guard.as_ref() {
                        match db.remove_user(target_id) {
                            Ok(true) => Ok(format!("✅ User {} removed", target_id)),
                            Ok(false) => Ok(format!("User {} not found", target_id)),
                            Err(e) => Ok(format!("Error: {}", e))
                        }
                    } else {
                        Ok("Database not initialized".to_string())
                    }
                }
                Some("info") => {
                    if parts.len() < 2 {
                        return Ok("Usage: /users info <telegram_id>".to_string());
                    }
                    let target_id = parts[1];
                    
                    let db_guard = DB.lock().unwrap();
                    if let Some(db) = db_guard.as_ref() {
                        match db.get_user_by_telegram_id(target_id) {
                            Ok(Some(user)) => Ok(format!(
                                "ℹ️ *User Info*\n\nID: {}\nUsername: @{}\nRole: {}\nJoined: {}",
                                user.telegram_id,
                                user.username.as_deref().unwrap_or("none"),
                                user.role,
                                user.created_at
                            )),
                            Ok(None) => Ok(format!("User {} not found", target_id)),
                            Err(e) => Ok(format!("Error: {}", e))
                        }
                    } else {
                        Ok("Database not initialized".to_string())
                    }
                }
                Some("setrole") | Some("role") => {
                    if parts.len() < 3 {
                        return Ok("Usage: /users setrole <telegram_id> <role>\nRoles: owner, admin, user, guest".to_string());
                    }
                    let target_id = parts[1];
                    let new_role = parts[2];
                    
                    // Prevent setting owner role - owner can only be set via BOT_OWNER_ID env var
                    if new_role == "owner" {
                        return Ok("❌ Cannot set owner role. Owner is set via BOT_OWNER_ID environment variable.".to_string());
                    }
                    
                    // Check if target is owner (from env)
                    if is_owner(target_id) {
                        return Ok("❌ Cannot modify owner role (set via BOT_OWNER_ID env var).".to_string());
                    }
                    
                    let db_guard = DB.lock().unwrap();
                    if let Some(db) = db_guard.as_ref() {
                        match db.update_user_role(target_id, new_role) {
                            Ok(true) => Ok(format!("✅ User {} role updated to {}", target_id, new_role)),
                            Ok(false) => Ok(format!("User {} not found", target_id)),
                            Err(e) => Ok(format!("Error: {}", e))
                        }
                    } else {
                        Ok("Database not initialized".to_string())
                    }
                }
                _ => Ok("Usage: /users <list|add|remove|info|setrole> [args]".to_string())
            }
        }));
}

/// Register /settings command for user personalization
fn register_settings_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    commands.register(Command::new("settings")
        .with_description("Manage your personal settings")
        .with_usage("/settings [get|set] [key] [value]")
        .with_handler(|msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            let user_id = &msg.chat_id;
            let args_str = args.join(" ");
            let parts: Vec<&str> = args_str.split_whitespace().collect();
            
            let db_guard = DB.lock().unwrap();
            let db = match db_guard.as_ref() {
                Some(db) => db,
                None => return Ok("Database not initialized".to_string())
            };
            
            match parts.first().map(|s| *s) {
                Some("get") | None => {
                    // Show current settings
                    match db.get_user_settings(user_id) {
                        Ok(Some(settings)) => {
                            Ok(format!(
                                "⚙️ *Your Settings*\n\n\
                                🌍 Language: {}\n\
                                🕐 Timezone: {}\n\
                                📝 Custom Prompt: {}\n\n\
                                _Use /settings set <key> <value> to change_",
                                settings.language,
                                settings.timezone,
                                settings.system_prompt.as_deref().unwrap_or("(none)")
                            ))
                        }
                        Ok(None) => {
                            Ok("⚙️ *Your Settings*\n\nUsing defaults:\n\
                                🌍 Language: en\n\
                                🕐 Timezone: UTC\n\
                                📝 Custom Prompt: (none)\n\n\
                                _Use /settings set <key> <value> to customize_".to_string())
                        }
                        Err(e) => Ok(format!("Error: {}", e))
                    }
                }
                Some("set") => {
                    if parts.len() < 3 {
                        return Ok("Usage: /settings set <key> <value>\n\n\
                                Keys:\n\
                                • language - en, id, jv\n\
                                • timezone - UTC, Asia/Jakarta, etc.\n\
                                • prompt - custom system prompt".to_string());
                    }
                    
                    let key = parts[1];
                    let value = parts[2..].join(" ");
                    
                    // Get current settings
                    let current = db.get_user_settings(user_id).ok().flatten()
                        .unwrap_or_else(|| database::UserSettings {
                            language: "en".to_string(),
                            timezone: "UTC".to_string(),
                            system_prompt: None,
                            preferences: "{}".to_string(),
                        });
                    
                    let mut settings = current;
                    
                    match key {
                        "language" | "lang" => {
                            if !["en", "id", "jv"].contains(&value.as_str()) {
                                return Ok("Invalid language. Use: en, id, jv".to_string());
                            }
                            settings.language = value.to_string();
                        }
                        "timezone" | "tz" => {
                            settings.timezone = value.to_string();
                        }
                        "prompt" | "system" => {
                            settings.system_prompt = Some(value.to_string());
                        }
                        _ => return Ok("Invalid key. Use: language, timezone, prompt".to_string()),
                    }
                    
                    if let Err(e) = db.set_user_settings(user_id, &settings) {
                        return Ok(format!("Error saving: {}", e));
                    }
                    
                    Ok(format!("✅ Setting saved:\n{} = {}", key, value))
                }
                _ => Ok("Usage: /settings <get|set> [key] [value]".to_string())
            }
        }));
}

fn generate_greeting(bot_username: &str, lang: &str) -> String {
    match lang {
        "jv" => generate_javanese_greeting(bot_username),
        "id" => generate_indonesian_greeting(bot_username),
        _ => generate_english_greeting(bot_username),
    }
}

fn generate_english_greeting(_bot_username: &str) -> String {
    "Hello! Welcome to Carik Bot 👋\n\n\
I'm your AI assistant. How can I help you today?\n\n\
/help - Get help\n\
/about - About Carik\n\
/ping - Ping\n\
/settings - Your settings\n\
/quote - Random quote".to_string()
}

fn generate_indonesian_greeting(_bot_username: &str) -> String {
    "Halo! Selamat datang di Carik Bot 👋\n\n\
Saya asisten AI Anda. Ada yang bisa saya bantu?\n\n\
/help - Bantuan\n\
/about - Tentang Carik\n\
/ping - Ping\n\
/settings - Pengaturan Anda\n\
/quote - Kutipan acak".to_string()
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

fn register_rss_command(commands: &mut CommandService) {
    use crate::domain::entities::{Command, Content};
    
    // RSS feed presets
    let feeds = vec![
        ("yahoo", "Yahoo News", "https://news.yahoo.com/rss/topstories"),
        ("google", "Google News", "https://news.google.com/rss"),
        ("bbc", "BBC News", "http://feeds.bbci.co.uk/news/rss.xml"),
        ("techcrunch", "TechCrunch", "https://techcrunch.com/feed/"),
        ("hn", "Hacker News", "https://hnrss.org/newest"),
    ];
    
    // Main rss command
    commands.register(Command::new("rss")
        .with_description("Fetch RSS feeds")
        .with_usage("/rss [feed_name|list|URL]")
        .with_handler(move |msg| {
            let Content::Command { name: _, args } = &msg.content else {
                return Ok("Error: invalid command".to_string());
            };
            
            if args.is_empty() {
                // Fetch default feed using blocking reqwest
                let url = "https://news.yahoo.com/rss/topstories".to_string();
                return Ok(fetch_rss_feed_blocking(&url));
            }
            
            let query = args.join(" ").to_lowercase();
            
            if query == "list" {
                let list: Vec<String> = feeds.iter()
                    .map(|(key, name, _)| format!("• {} - {}", name, key))
                    .collect();
                return Ok(format!("📡 Available Feeds:\n\n{}\n\nUsage: /rss <name>", list.join("\n")));
            }
            
            // Find matching feed
            let matched = feeds.iter()
                .find(|(key, name, _)| key.contains(&query) || name.to_lowercase().contains(&query));
            
            let url = matched.map(|(_, _, url)| url.to_string()).unwrap_or(query);
            
            Ok(fetch_rss_feed_blocking(&url))
        }));
}

fn fetch_rss_feed_blocking(url: &str) -> String {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build() 
    {
        Ok(c) => c,
        Err(e) => return format!("❌ Client error: {}", e),
    };
    
    match client.get(url)
        .header("User-Agent", "CarikBot/1.0")
        .send() 
    {
        Ok(response) => {
            match response.bytes() {
                Ok(bytes) => {
                    match rss::Channel::read_from(&bytes[..]) {
                        Ok(channel) => {
                            let title = channel.title();
                            let items: Vec<String> = channel.items().iter()
                                .take(5)
                                .map(|item| {
                                    let title = item.title().unwrap_or("No title");
                                    let link = item.link().unwrap_or("");
                                    format!("📰 {}\n🔗 {}", title, link)
                                })
                                .collect();
                            
                            format!("📡 *{}*\n\n{}", 
                                title.replace('*', "\\*"),
                                items.join("\n\n")
                            )
                        }
                        Err(e) => format!("❌ Parse error: {}", e)
                    }
                }
                Err(e) => format!("❌ Read error: {}", e)
            }
        }
        Err(e) => format!("❌ Fetch error: {}", e)
    }
}

fn init_config() {
    let config = Config::default();
    let yaml = serde_yaml::to_string(&config).unwrap();
    println!("{}", yaml);
    println!("\nSave this to config.yaml and adjust as needed.");
}
