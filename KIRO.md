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
| `/kiro-model [auto\|pro\|express]` | Switch Kiro model |

## Examples

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
/kiro-model pro
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
- Conversation history maintained in container
- Use `/kiro-new` to start fresh conversation

## Roadmap

- [x] Basic kiro execution
- [x] Session management (/kiro-new, /kiro-kill)
- [x] File operations (/kiro-read, /kiro-write, /kiro-ls)
- [ ] Git integration
- [ ] MCP tools
- [ ] Agent presets
