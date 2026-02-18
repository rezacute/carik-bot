# LLM Integration Plan

## Overview
Route user messages to appropriate handlers:
- **Coding tasks** → Kiro (AI coding agent)
- **Skill-based tasks** → Read skill.md and execute
- **General conversation** → LLM (Groq)

## Implementation

### 1. Coding Intent Detection
Keywords that trigger Kiro:
- write code, create app, build, make
- debug, fix, error, bug
- refactor, optimize
- code, program, script
- function, class, algorithm
- python, javascript, rust, etc.

Regex pattern: `(?i)(code|write|create|build|make|debug|fix|program|script|function|class)`

### 2. Skill Detection
Match user query against skill descriptions in `/skills/*/SKILL.md`

Detection:
- Parse skill name from folder
- Read SKILL.md description field
- Use keyword matching or LLM to detect best skill

### 3. Flow
```
User Message
    ↓
Is it a command? → Yes → Execute command
    ↓ No
Is it coding? → Yes → Route to Kiro
    ↓ No
Is it a skill? → Yes → Load skill.md → Execute
    ↓ No
Route to LLM (Groq)
```

### 4. Configuration
Add to config.yaml:
```yaml
llm:
  auto_route:
    coding_keywords: [code, write, create, build, debug, fix, ...]
    skills_dir: /path/to/skills
```

### 5. Code Changes
- Add `detect_intent()` function
- Add `route_to_kiro()` function
- Add `load_skill()` function
- Modify message handler to check intent before LLM

### 6. Example
User: "write a python script to fetch data"
→ Detected: coding intent
→ Route to Kiro with prompt

User: "what's the weather"
→ No skill match
→ Route to LLM

User: "create a note in obsidian"
→ Skill detected: obsidian
→ Load obsidian SKILL.md
→ Execute skill handler
