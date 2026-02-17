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
                                    let cmd_name = trimmed
                                        .split_whitespace()
                                        .next()
                                        .unwrap_or("");
                                    
                                    let msg = Message::from_command(&chat_id, cmd_name, vec![]);
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
/// Execute kiro-cli as a coding agent
async fn execute_kiro_cli(prompt: &str) -> String {
    use tokio::process::Command;
    use std::io::Write;
    
    tracing::info!("Executing kiro-cli with prompt: {}", prompt);
    
    let mut child = Command::new("/home/ubuntu/.local/bin/kiro-cli")
        .arg("chat")
        .arg("--no-interactive")
        .arg("--trust-all-tools")
        .arg(&prompt)
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
