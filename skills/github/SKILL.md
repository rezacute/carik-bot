---
name: carik-github
description: Git operations for carik-bot - commit, push, status, and repo management.
metadata:
  {
    "carik": { "emoji": "🐙", "requires": { "bins": ["git"] } },
    "openclaw": { "emoji": "🐙", "requires": { "bins": ["git"] } }
  }
---

# GitHub Skill for Carik Bot

Git operations via command line.

## Common operations

### Check git status

```bash
cd /path/to/repo && git status
```

### Stage and commit

```bash
cd /path/to/repo && git add -A && git commit -m "message"
```

### Push to remote

```bash
cd /path/to/repo && git push origin main
```

### View recent commits

```bash
cd /path/to/repo && git log --oneline -5
```

### Pull latest changes

```bash
cd /path/to/repo && git pull origin main
```

## Carik bot conventions

- Always commit with descriptive messages
- Use present tense: "Add feature" not "Added feature"
- Group related changes in single commits

## Example usage

```
User: Commit my changes
Carik: What commit message would you like?
```

```
User: Check repo status
Carik: On branch main, clean working tree ✓
```
