use clap::{Parser, Subcommand};
use tracing_subscriber;

mod domain;
mod application;
mod infrastructure;

use infrastructure::config::Config;
use infrastructure::adapters::telegram::TelegramAdapter;
use infrastructure::adapters::console::ConsoleAdapter;
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
        rt.block_on(async {
            let bot = TelegramAdapter::new(token);
            run_bot_async(bot, commands).await;
        });
    } else {
        // Run console bot (dev mode)
        rt.block_on(async {
            let bot = ConsoleAdapter::new();
            run_bot_async(bot, commands).await;
        });
    }
}

async fn run_bot_async<B: Bot>(bot: B, mut commands: CommandService) {
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
