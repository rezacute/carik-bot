# LLM Integration Plan

## Overview
Route user messages to appropriate handlers:
- **Coding tasks** → Kiro (AI coding agent)
- **Skill-based tasks** → Read skill.md and execute
- **Plugin tasks** → MCP/A2A/Wasm plugins
- **General conversation** → LLM (Groq)

---

## Plugin Integration with LLM (Butler Service)

The bot acts as a "Butler" - it knows its capabilities (plugins) and can delegate tasks appropriately.

### Plugin Context for LLM

When the LLM processes a message, it should know what plugins are available:

```rust
/// Get plugin context for LLM prompts
fn get_plugin_context(manager: &PluginManager) -> String {
    let plugins = manager.list_plugins();
    
    if plugins.is_empty() {
        return String::new();
    }
    
    let mut context = String::from("## Available Plugins (Butler Tools)\n\n");
    
    for plugin in &plugins {
        context.push_str(&format!(
            "- **{}**: {}\n",
            plugin.name,
            plugin.description
        ));
    }
    
    context.push_str("\nUse these plugins to help the user. ");
    context.push_str("When a plugin executes, acknowledge it and integrate results naturally.\n");
    
    context
}
```

### Acknowledging Plugin Execution

When a plugin runs, include its output in LLM context with acknowledgment:

```rust
async fn execute_with_llm_acknowledgment(
    plugin_name: &str,
    args: Value,
    manager: &PluginManager,
) -> String {
    let result = manager.execute(plugin_name, args).await;
    
    match result {
        Ok(output) => {
            // Format for LLM to acknowledge and integrate
            format!(
                "[Plugin '{}' executed successfully]\nResult: {}\n[End plugin result]\n\
                Acknowledge this result and integrate it naturally into your response.",
                plugin_name,
                serde_json::to_string_pretty(&output).unwrap_or_default()
            )
        }
        Err(e) => {
            format!(
                "[Plugin '{}' failed: {}]\n\
                Acknowledge the error and suggest alternatives if possible.",
                plugin_name,
                e
            )
        }
    }
}
```

### Butler Service Prompt

Add to system prompt for LLM awareness:

```yaml
butler:
  # Butler service configuration
  name: "Carik"
  description: "Your personal AI assistant"
  
  # What the butler knows about itself
  capabilities:
    - name: "rss"
      description: "Fetch news from RSS feeds"
      example: "Get me the latest tech news"
    - name: "kiro"
      description: "AI coding assistant"
      example: "Write me a Python script"
    - name: "filesystem"
      description: "Read/write files"
      example: "Create a note"
```

### LLM System Prompt Addition

```
## Butler Service Mode

You are Carik, a helpful personal assistant ("Butler"). You have access to plugins 
that extend your capabilities. When a plugin is used:

1. **Acknowledge** the plugin execution to the user
2. **Integrate** the results naturally into your response
3. **Explain** what you did if relevant

Example:
- User: "What's the latest news?"
- You: "I'll fetch the latest news for you." [uses RSS plugin]
- Plugin result arrives
- You: "Here's the latest news from Yahoo: [headlines...]"

Available plugins are listed in your context. Use them proactively to help users.
```

### Plugin-Intent Detection

Automatically route to plugins based on user intent:

```rust
fn detect_plugin_intent(text: &str, plugins: &[PluginInfo]) -> Option<(&str, Value)> {
    let lower = text.to_lowercase();
    
    for plugin in plugins {
        let triggers = match plugin.name.as_str() {
            "rss" => vec!["news", "feed", "headlines", "latest"],
            "kiro" => vec!["code", "write", "create", "build", "debug"],
            "weather" => vec!["weather", "temperature", "forecast"],
            _ => vec![],
        };
        
        for trigger in triggers {
            if lower.contains(trigger) {
                return Some((plugin.name.as_str(), json!({})));
            }
        }
    }
    
    None
}
```

### Updated Message Flow

```
User Message
    ↓
Is it a command? → Yes → Execute command
    ↓ No
Detect plugin intent → Match → Execute plugin → Acknowledge & integrate result
    ↓ No match
Is it coding? → Yes → Route to Kiro
    ↓ No
Is it a skill? → Yes → Load skill.md → Execute
    ↓ No
Route to LLM (with plugin context)
```

### Configuration

```yaml
llm:
  auto_route:
    coding_keywords: [code, write, create, build, debug, fix, ...]
    skills_dir: /path/to/skills
  
  # Butler service settings
  butler:
    enabled: true
    acknowledge_plugins: true
    include_plugin_context: true
```

---

## Original Implementation

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
