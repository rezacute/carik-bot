---
name: carik-help
description: Help and skill management for carik-bot.
metadata:
  {
    "carik": { "emoji": "❓", "requires": {} },
    "openclaw": { "emoji": "❓", "requires": {} }
  }
---

# Help Skill for Carik Bot

Manage and display available skills and commands.

## Available commands

| Command | Description |
|---------|-------------|
| `/help` | Show help message |
| `/about` | About carik-bot |
| `/ping` | Check bot is alive |
| `/skills` | List all skills |
| `/quote` | Get inspirational quote |
| `/weather [location]` | Get weather |
| `/clear` | Clear conversation |

## Skill system

Carik-bot uses a skill-based architecture:

```
skills/
├── weather/     - Weather queries
├── github/      - Git operations
├── quotes/      - Inspirational quotes
└── ...
```

### Loading skills

Skills are loaded from `skills/` directory at startup.

### Skill format

Each skill has `SKILL.md` with:
- Name and description
- Usage examples
- Format codes
- Integration notes

## Example conversation

```
User: What can you do?
Carik: I'm Carik, your AI assistant! I can:
• Answer questions
• Help with git operations
• Share quotes
• Check weather
• And more!

Type /help for commands.
```
