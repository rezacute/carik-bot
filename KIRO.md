# Kiro Integration

## Commands

| Command | Description |
|---------|-------------|
| `/kiro <prompt>` | Run kiro with a prompt |
| `/kiro-status` | Check if kiro is running |
| `/kiro-log` | View last output |
| `/kiro-kill` | Stop kiro session |
| `/kiro-new` | Start new conversation |
| `/kiro-ls` | List workspace files |
| `/kiro-read <file>` | Read file from workspace |
| `/kiro-write <file> <content>` | Write file to workspace |
| `/kiro-model [auto\|opus\|sonnet\|haiku]` | Switch Kiro model |
| `/kiro-fresh` | Start fresh conversation (clear history) |

## Session Persistence

Kiro now automatically resumes the last conversation!

- `/kiro <prompt>` - Automatically resumes previous conversation
- `/kiro-fresh` - Start a completely new conversation

The conversation is persisted in the Docker container and survives until you kill it with `/kiro-kill`.

### Run a prompt
```
/kiro write a hello world in python
```

### List files
```
/kiro-ls
```

### Read file
```
/kiro-read main.rs
```

### Write file
```
/kiro-write test.txt Hello World
```

### Switch model
```
/kiro-model opus
```

### Start new conversation
```
/kiro-new
```

## Docker

Kiro runs in a Docker container (`kiro-persistent`) for isolation.

The container has access to:
- `~/.kiro` - Kiro config and data
- `~/.aws` - AWS credentials
- `~/.local/bin/kiro-cli` - Kiro CLI binary
- `/workspace/default-workspace` - Bot workspace

## Architecture

```
Telegram -> carik-bot -> Docker (kiro-persistent) -> kiro-cli
```

## Session Management

- Container persists until explicitly killed
- Conversation history is automatically resumed between commands
- Use `/kiro-fresh` to start a new conversation without history
- Use `/kiro-kill` to stop the session

## Roadmap

- [x] Basic kiro execution
- [x] Session management (/kiro-new, /kiro-kill)
- [x] File operations (/kiro-read, /kiro-write, /kiro-ls)
- [ ] Git integration
- [ ] MCP tools
- [ ] Agent presets
