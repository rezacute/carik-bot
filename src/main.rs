use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "carik-bot")]
#[command(about = "A Rust CLI bot", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the bot
    Run {
        /// Bot token
        #[arg(long)]
        token: Option<String>,
    },
    /// Show version
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { token } => {
            println!("Starting carik-bot...");
            if let Some(t) = token {
                println!("Token: {}", t);
            }
        }
        Commands::Version => {
            println!("carik-bot v{}", env!("CARGO_PKG_VERSION"));
        }
    }
}
