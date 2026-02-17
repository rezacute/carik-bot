# carik-bot

A minimal, secure bot framework with clean architecture — inspired by OpenClaw but streamlined.

## Features

- **Clean Architecture** — Domain, Application, Infrastructure layers
- **Plugin System** — Hot-loadable plugins with permissions
- **Multiple Adapters** — Telegram, Console (dev mode)
- **Config Management** — YAML + environment variables
- **Error Handling** — Structured error types with `thiserror`
- **Logging** — `tracing` for structured logging

## Architecture

```
src/
├── domain/              # Core business logic (no external deps)
│   ├── entities/       # User, Message, Command
│   └── traits/         # Bot, Store abstractions
├── application/        # Use cases
│   ├── errors.rs      # Domain errors (BotError, CommandError, etc.)
│   └── services/      # CommandService, MessageService
├── infrastructure/     # External concerns
│   ├── config/        # YAML + env config
│   ├── storage/       # JSON file store
│   ├── adapters/      # Telegram, Console
│   └── plugins/       # Plugin system
└── presentation/
    └── main.rs        # CLI entry
```

## Quick Start

```bash
# Build
cargo build --release

# Run in console mode (dev)
cargo run

# Run with Telegram token
BOT_TOKEN=your_token cargo run

# Show version
cargo run -- version

# Generate default config
cargo run -- init-config
```

## Configuration

Create `config.yaml`:

```yaml
bot:
  name: carik-bot
  prefix: "!"

plugins:
  directory: ./plugins
  auto_load: true

security:
  rate_limit:
    max_requests: 20
    window_seconds: 60
  sandbox:
    enabled: true

adapters:
  telegram:
    enabled: true
    token: ${BOT_TOKEN}
```

Or use environment variables:
- `BOT_TOKEN` — Telegram bot token
- `BOT_PREFIX` — Command prefix (default: `/`)

## Plugin System

Plugins are dynamically loaded from the `plugins/` directory.

### Plugin Structure

```
plugins/hello/
├── plugin.toml    # Required manifest
└── libhello.so   # Compiled plugin (optional if using default naming)
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

### Available Permissions

- `read-messages` — Read incoming messages
- `send-messages` — Send messages
- `manage-commands` — Register bot commands
- `filesystem` — Access file system
- `http` — Make HTTP requests
- `env-vars` — Access environment variables
- `load-plugins` — Load other plugins

### Writing a Plugin

```rust
use carik_bot::plugins::Plugin;

struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn init(&self) -> carik_bot::PluginResult<()> {
        tracing::info!("Hello plugin initialized!");
        Ok(())
    }
    
    fn name(&self) -> &str { "hello" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> Option<&str> { Some("A hello world plugin") }
    
    fn shutdown(&self) -> carik_bot::PluginResult<()> {
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn carik_plugin_init() -> *mut dyn Plugin {
    Box::into_raw(Box::new(HelloPlugin))
}
```

## Commands

Built-in commands:
- `/help` — Show help message
- `/version` — Show bot version

## Roadmap to v1

- ✅ Phase 1: Clean Architecture
- ✅ Phase 2: Plugin System
- 🔄 Phase 3: Message Handling + Middleware
- ⏳ Phase 4: Platform Adapters
- ⏳ Phase 5: Security Hardening
- ⏳ Phase 6: CI/CD + Docker
- ⏳ Phase 7: Release v1.0.0

See [ROADMAP.md](./ROADMAP.md) for details.

## Dependencies

- `clap` — CLI argument parsing
- `tokio` — Async runtime
- `serde` / `serde_yaml` — Serialization
- `thiserror` — Error handling
- `tracing` — Logging
- `libloading` — Dynamic library loading

## License

MIT
