<img src="carik_logo.jpg" alt="carik-bot logo" width="200" align="left" style="margin-right: 20px; margin-bottom: 20px;">

# carik-bot

> **carik** (Javanese: ꦕꦫꦶꦏ) — A faithful Javanese butler for your digital life.

**carik** (pronounced "cha-reek") is a bot framework named after the Javanese word for a trusted household servant — someone who anticipates needs, handles tasks quietly, and serves with discretion.

Just as a **lurah** (village head) in Javanese culture relies on their trusted carik to manage household affairs, carik-bot serves as your reliable digital assistant.

---

## Features

- 🤖 **Telegram Integration** — Full Telegram Bot API support with webhook and long polling
- 🧠 **LLM Integration** — Groq-powered AI responses with conversation memory
- 🎯 **Command System** — Prefix-based commands with help auto-generation
- 🔌 **Plugin Architecture** — Hot-loadable plugins with permission system
- 🏗️ **Clean Architecture** — Domain, Application, Infrastructure layers
- 🔐 **Security Hardened** — systemd security options, non-root execution
- 📝 **Structured Logging** — tracing-based logging to journald
- ⚙️ **Config Management** — YAML + environment variables

## Quick Start

### Prerequisites

- Rust 1.70+
- Telegram Bot Token (from [@BotFather](https://t.me/BotFather))
- Optional: Groq API Key (for AI features)

### Build & Run

```bash
# Clone and build
git clone https://github.com/yourusername/carik-bot.git
cd carik-bot
cargo build --release

# Configure
cp .env.example .env
# Edit .env with your BOT_TOKEN and GROQ_API_KEY

# Run in console mode (dev)
cargo run

# Run with Telegram
./target/release/carik-bot run
```

### systemd Deployment

```bash
# Install as systemd service (auto-start on boot)
sudo ./scripts/install-systemd.sh

# Check status
systemctl status carik-bot

# View logs
journalctl -u carik-bot -f
```

See [DEPLOYMENT.md](./DEPLOYMENT.md) for detailed deployment notes.

## Commands

| Command | Description |
|---------|-------------|
| `/help` | Show help message |
| `/about` | About carik-bot |
| `/ping` | Pong! |
| `/clear` | Clear conversation history |
| `/quote` | Get a random quote |

## Configuration

### Environment Variables

```bash
BOT_TOKEN=your_telegram_bot_token_here
GROQ_API_KEY=your_groq_api_key_here  # Optional, for AI features
```

### config.yaml

```yaml
bot:
  name: carik-bot
  prefix: "!"

whitelist:
  enabled: false  # Set true to only allow specific users
  users:
    - "123456789"
```

## Architecture

```
src/
├── domain/              # Core business logic (no external deps)
│   └── entities/       # Message, Command, User
├── application/        # Use cases
│   ├── errors.rs      # Domain errors
│   └── services/      # CommandService
├── infrastructure/     # External concerns
│   ├── config/         # YAML + env config
│   ├── adapters/       # Telegram, Console
│   └── llm/           # Groq LLM provider
└── main.rs             # CLI entry point
```

## Plugins

Plugins are dynamically loaded from the `plugins/` directory.

```
plugins/hello/
├── plugin.toml    # Required manifest
└── libhello.so   # Compiled plugin
```

### plugin.toml

```yaml
name: hello
version: "0.1.0"
description: A hello world plugin
permissions:
  - read-messages
  - send-messages
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust 1.70+ |
| Async | Tokio |
| Telegram | reqwest + serde |
| LLM | Groq API |
| Config | serde_yaml |
| Logging | tracing + journald |
| CLI | clap |

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a PR

## License

MIT

---

**carik-bot** — Your faithful digital servant.
