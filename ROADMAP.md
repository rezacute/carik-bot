# carik-bot Roadmap

## Current Version: v0.1.0

---

## Completed Features

### ✅ Core Architecture
- [x] Clean architecture (Domain, Application, Infrastructure)
- [x] Telegram adapter with long polling
- [x] Command system with prefix
- [x] YAML + env config
- [x] tracing-based logging

### ✅ User Management (RBAC)
- [x] SQLite database (`carik-bot.db`)
  - `users` table (telegram_id, username, role)
  - `rate_limits` table (user_id, timestamp, query_type)
- [x] Roles: owner, admin, user, guest
- [x] `/connect` - Guest access request
- [x] `/approve <id>` - Owner approves guests
- [x] `/users` - User management (owner/admin)

### ✅ Rate Limiting
- [x] 1 query per minute per user
- [x] 20 queries per hour per user
- [x] Owner exempt from rate limiting

### ✅ Kiro Integration
- [x] Docker container for kiro-cli (`kiro-persistent`)
- [x] `/code` - Run kiro-cli as coding agent
- [x] `/kiro` - Run kiro in Docker with chat
- [x] `/kiro-status` - Check if running
- [x] `/kiro-log` - View output
- [x] `/kiro-kill` - Stop session

### ✅ Workspace Management
- [x] `/workspace` - Manage workspaces
- [x] `.carik-bot/` home directory
- [x] Multiple workspace support

### ✅ UI/UX
- [x] Javanese greeting (`/start`)
- [x] MarkdownV2 support for Telegram
- [x] Typing indicator
- [x] Fallback to plain text on errors

### ✅ Plugin Architecture
- [x] Plugin trait and manager
- [x] MCP placeholder
- [x] A2A placeholder
- [x] RSS plugin with /rss command

### ✅ Butler Service (LLM Integration)
- [x] Plugin context provider for LLM
- [x] Butler system prompt
- [x] Config: butler section
- [x] Intent detection for auto-routing
- [x] Plugin result acknowledgment

---

## In Progress

### 🔄 Telegram Polling Issue
- Bot sometimes doesn't receive messages (409 Conflict)
- Needs investigation

---

## Upcoming Features

### v0.2.0 - User Management Enhanced
- [ ] `/users add <id> <role>` - Add user with role
- [ ] `/users remove <id>` - Remove user
- [ ] Username tracking
- [ ] User activity logging

### v0.3.0 - LLM Enhancements
- [ ] Conversation memory persistence
- [ ] Multiple LLM providers (Claude, MiniMax)
- [ ] Streaming responses

### v1.0.0 - Production Release
- [ ] Security audit
- [ ] Docker container for bot
- [ ] CI/CD pipeline
- [ ] Complete documentation

---

## Configuration Example

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

security:
  rate_limit:
    max_requests: 20
    window_seconds: 60
  sandbox:
    enabled: false

adapters:
  telegram:
    enabled: true
    token: ${BOT_TOKEN}
```

---

## Database Schema

```sql
-- Users table
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    telegram_id TEXT UNIQUE NOT NULL,
    username TEXT,
    role TEXT NOT NULL DEFAULT 'guest',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Rate limits table
CREATE TABLE rate_limits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    query_type TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

---

## Commands Matrix

| Command | Owner | Admin | User | Guest |
|---------|-------|-------|------|-------|
| /start | ✅ | ✅ | ✅ | ✅ |
| /help | ✅ | ✅ | ✅ | ✅ |
| /ping | ✅ | ✅ | ✅ | ✅ |
| /about | ✅ | ✅ | ✅ | ✅ |
| /clear | ✅ | ✅ | ✅ | ✅ |
| /quote | ✅ | ✅ | ✅ | ✅ |
| /connect | - | - | - | ✅ |
| /approve | ✅ | ❌ | ❌ | ❌ |
| /users | ✅ | ✅ | ❌ | ❌ |
| /workspace | ✅ | ✅ | ✅ | ✅ |
| /code | ✅ | ✅ | ✅ | ❌ |
| /kiro | ✅ | ✅ | ✅ | ❌ |
| /rss | ✅ | ✅ | ✅ | ❌ |
