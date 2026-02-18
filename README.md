<img src="carik_logo.jpg" alt="carik-bot logo" width="200" align="left" style="margin-right: 20px; margin-bottom: 20px;">

# carik-bot

> **carik** (Javanese: ꦕꦫꦶꦏ) — A faithful Javanese butler for your digital life.

**carik** (pronounced "cha-reek") is a Telegram bot with AI capabilities, named after the Javanese word for a trusted household servant.

---

## Features

- 🤖 **Telegram Integration** — Long polling Bot API support
- 🧠 **LLM Integration** — Groq-powered AI responses with conversation memory
- 🎯 **Command System** — Prefix-based commands with help auto-generation
- 🔐 **RBAC** — Owner/Admin/User/Guest roles with SQLite database
- 📊 **Rate Limiting** — 1 query/minute, 20 queries/hour per user
- 🔌 **Docker Support** — Kiro CLI runs in Docker container for isolation
- 🏗️ **Clean Architecture** — Domain, Application, Infrastructure layers
- ⚙️ **Config Management** — YAML + environment variables

## Commands

| Command | Description | Access |
|---------|-------------|--------|
| `/start` | Show Javanese greeting | All |
| `/help` | Show help message | All |
| `/ping` | Pong! | All |
| `/about` | About carik-bot | All |
| `/clear` | Clear conversation history | All |
| `/quote` | Get a random quote | All |
| `/connect` | Request guest access | Guest |
| `/approve <id>` | Approve guest (owner) | Owner |
| `/users` | Manage users | Owner/Admin |
| `/workspace` | Manage workspaces | All |
| `/code` | Run kiro-cli coding agent | Approved |
| `/kiro` | Run kiro in Docker | Approved |

## Quick Start

### Prerequisites

- Rust 1.70+
- Telegram Bot Token (from [@BotFather](https://t.me/BotFather))
- Optional: Groq API Key (for AI features)

### Build & Run

```bash
# Clone and build
git clone https://github.com/rezacute/carik-bot.git
cd carik-bot
cargo build --release

# Configure
cp .env.example .env
# Edit .env with your BOT_TOKEN and GROQ_API_KEY

# Run with Telegram
./target/release/carik-bot run
```

### systemd Deployment

```bash
# Install as systemd service
sudo cp carik-bot.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable carik-bot
sudo systemctl start carik-bot

# Check status
systemctl status carik-bot

# View logs
journalctl -u carik-bot -f
```

## Configuration

### config.yaml

```yaml
bot:
  name: carik-bot
  prefix: "!"

whitelist:
  enabled: true
  users:
    - "6504720757"

guests:
  enabled: true
  pending: []
  approved: []
```

### Environment Variables

```bash
BOT_TOKEN=your_telegram_bot_token_here
GROQ_API_KEY=your_groq_api_key_here
```

## User Management

### Roles

- **owner** - Full access, can manage users
- **admin** - Can manage users, all commands
- **user** - Regular access
- **guest** - Limited, needs approval

### Flow

1. **Guest** sends `/connect` → request goes to pending
2. **Owner** runs `/approve <user_id>` → user approved
3. **Approved user** can use `/code` and `/kiro`

### Rate Limiting

- **1 query per minute** per user
- **20 queries per hour** per user
- Owner is exempt from rate limiting

## Architecture

```
src/
├── domain/              # Core business logic
│   ├── entities/       # Message, Command, User
│   └── traits/         # Bot trait
├── application/        # Use cases
│   ├── errors.rs       # Domain errors
│   └── services/       # CommandService
├── infrastructure/     # External concerns
│   ├── config/        # YAML config
│   ├── database/      # SQLite (users, rate limits)
│   ├── adapters/       # Telegram, Console
│   └── llm/           # Groq LLM provider
└── main.rs             # CLI entry point
```

## Docker

Kiro CLI runs in a Docker container for isolation:

```bash
# Container is auto-created
docker ps | grep kiro-persistent
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust 1.70+ |
| Async | Tokio |
| Telegram | reqwest + serde |
| LLM | Groq API |
| Database | SQLite (rusqlite) |
| Config | serde_yaml |
| Logging | tracing + journald |
| CLI | clap |

## License

MIT
