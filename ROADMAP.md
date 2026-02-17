# carik-bot Roadmap to v1

## Overview
A minimal, secure bot framework with clean architecture — inspired by OpenClaw but streamlined.

---

## v1.0.0 Milestones

### Phase 1: Core Architecture (Week 1-2)
**Goal:** Establish clean architecture foundation

```
src/
├── domain/           # Core business logic (no external deps)
│   ├── entities/     # User, Message, Command, Plugin
│   └── traits/       # abstractions (Bot trait, Store trait)
├── application/     # Use cases
│   ├── commands/     # CLI command handlers
│   ├── services/     # Business logic services
│   └── errors/       # Domain errors
├── infrastructure/   # External concerns
│   ├── config/       # Config loading (env, yaml, toml)
│   ├── storage/      # File/JSON persistence
│   └── http/         # HTTP client if needed
└── presentation/     # CLI entry point
    └── main.rs
```

**Deliverables:**
- [ ] Project structure with clean architecture
- [ ] Basic entity definitions
- [ ] Error handling enum
- [ ] Config loader (env + file)

---

### Phase 2: Plugin System (Week 3)
**Goal:** Hot-loadable skill/plugin system

**Features:**
- Plugin discovery from `plugins/` directory
- Trait-based plugin interface
- Sandboxed execution (optional: wasm, isolate)
- Plugin metadata (`plugin.toml`)

**Security:**
- Plugin permission system
- No `unsafe` in plugins by default
- Resource limits (time, memory)

**Deliverables:**
- [ ] `Plugin` trait definition
- [ ] Plugin loader (dynamic `libloading`)
- [ ] `plugin.toml` schema
- [ ] Permission config

---

### Phase 3: Message Handling (Week 4)
**Goal:** Process incoming messages/commands

**Features:**
- Message parsing (text, commands, callbacks)
- Event-driven architecture
- Middleware pipeline (auth → ratelimit → handler)
- Response routing

**Deliverables:**
- [ ] Message types (Text, Command, Callback)
- [ ] Middleware system (stackable)
- [ ] Command dispatcher
- [ ] Basic rate limiter

---

### Phase 4: Platform Adapters (Week 5)
**Goal:** Support multiple messaging platforms

**Adapters (MVP):**
- [ ] Telegram bot API
- [ ] Console/CLI (dev mode)

**Architecture:**
```
infrastructure/
└── adapters/
    ├── telegram/
    ├── discord/
    └── console/
```

**Deliverables:**
- [ ] Telegram adapter
- [ ] Adapter trait
- [ ] Platform-agnostic message conversion

---

### Phase 5: Security Hardening (Week 6)
**Goal:** Outstanding security posture

**Security Features:**
- [ ] Secrets management (no plain text tokens)
- [ ] Input sanitization (XSS, injection)
- [ ] Rate limiting per user/chat
- [ ] Audit logging
- [ ] TLS/HTTPS for webhooks
- [ ] Plugin sandboxing (firejail or similar)

**Security Config:**
```yaml
security:
  rate_limit:
    max_requests: 20
    window_seconds: 60
  sandbox:
    enabled: true
    memory_mb: 256
  audit:
    enabled: true
    path: logs/audit.log
```

---

### Phase 6: v1 Release (Week 7)
**Goal:** Production-ready v1.0.0

**Deliverables:**
- [ ] CI/CD pipeline
- [ ] Docker container
- [ ] Documentation
- [ ] Versioning scheme
- [ ] Changelog

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                   Presentation                       │
│                  (CLI, main.rs)                      │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│                   Application                       │
│           (Commands, Services, Errors)              │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│                     Domain                          │
│            (Entities, Traits, Rules)                │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│                  Infrastructure                     │
│    (Config, Storage, Adapters, Security)           │
└─────────────────────────────────────────────────────┘
```

---

## v1 Configuration Example

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

---

## Dependencies (Recommended)

```toml
[dependencies]
# CLI
clap = { version = "4", features = ["derive"] }

# Async
tokio = { version = "1", features = ["full"] }

# Config
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
config = "0.14"

# Plugin system
libloading = "0.8"

# Security
ring = "0.17"
rustls = "0.23"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Utils
thiserror = "1"
async-trait = "0.1"
```

---

## Next Steps

1. **Approve architecture** — Confirm structure above
2. **Start Phase 1** — Set up clean architecture folders
3. **Define entities** — What does the bot manipulate?

Let me know when ready to start! 🚀
