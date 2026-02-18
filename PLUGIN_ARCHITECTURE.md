# Carik-Bot Plugin Architecture Plan

## Overview

Extend carik-bot with a modular plugin system supporting:
- **MCP** (Model Context Protocol) - External tools/services
- **A2A** (Agent-to-Agent) - Communication between agents
- **Wasm** (WebAssembly) - User-defined plugins

---

## Phase 1: Foundation ✅ IMPLEMENTED

### Completed

1. **Plugin Trait** (`src/plugins/trait_def.rs`)
   - `Plugin` trait with `name()`, `description()`, `execute()`, `cleanup()`
   - `PluginKind` enum for MCP/A2A/Wasm
   - `ExtendedPluginConfig` for runtime config
   - `PluginResult` for execution results

2. **Plugin Manager** (`src/plugins/manager.rs`)
   - `PluginManager` struct with registry
   - `register()`, `unregister()`, `execute()` methods
   - `list_plugins()` for listing
   - Thread-safe wrapper with `SharedPluginManager`

3. **Module Structure** (`src/plugins/mod.rs`)
   - `plugins/mod.rs` - Module exports
   - `plugins/mcp/mod.rs` - MCP placeholder
   - `plugins/a2a/mod.rs` - A2A placeholder
   - Integrated into `main.rs`

### Verification

```
Feb 18 22:54:44 carik-bot[3415264]: INFO Plugin system initialized with 0 plugins
```

---

## 2. MCP Integration (Model Context Protocol)

### Core Components

```
┌─────────────────────────────────────────────────────────┐
│                    Carik-Bot Core                       │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Plugin    │  │   Plugin    │  │   Plugin    │   │
│  │   Manager   │  │   Manager   │  │   Manager   │   │
│  └─────────────┘  └─────────────┘  └─────────────┘   │
│         ↓                ↓                ↓            │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Plugin Interface                    │   │
│  │  - init()    - execute()    - cleanup()       │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
         ↓                   ↓                   ↓
    ┌─────────┐        ┌─────────┐        ┌─────────┐
    │   MCP   │        │   A2A   │        │  Wasm   │
    │ Plugins │        │ Plugins │        │ Plugins │
    └─────────┘        └─────────┘        └─────────┘
```

### Plugin Trait

```rust
trait Plugin {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: Value) -> Result<Value, String>;
    fn cleanup(&self) {}
}
```

---

## 2. MCP Integration (Model Context Protocol)

### Concept
MCP allows carik-bot to connect to external tools and services (databases, APIs, file systems).

### MCP Server Types
1. **Filesystem** - Read/write files
2. **Database** - SQL queries
3. **HTTP** - REST API calls
4. **Custom** - User-defined tools

### Implementation

```rust
// MCP Plugin Structure
struct McpPlugin {
    name: String,
    server_url: String,
    tools: Vec<McpTool>,
}

struct McpTool {
    name: String,
    description: String,
    input_schema: Value,
}

// Register MCP tool as bot command
fn register_mcp_tools(plugins: &[McpPlugin]) {
    for plugin in plugins {
        for tool in &plugin.tools {
            // Create command from MCP tool
            commands.register(Command::new(&tool.name)
                .with_description(&tool.description)
                .with_handler(move |msg| {
                    // Execute via MCP
                    execute_mcp_tool(&tool.name, &args).await
                }));
        }
    }
}
```

### Config (config.yaml)
```yaml
mcp:
  enabled: true
  servers:
    filesystem:
      type: filesystem
      root: /home/ubuntu/.carik-bot/plugins
    database:
      type: database
      connection: sqlite:carik-bot.db
    github:
      type: http
      url: https://api.github.com
      headers:
        Authorization: "Bearer $GITHUB_TOKEN"
```

### Commands
- `/mcp list` - List available MCP tools
- `/mcp status` - Check MCP server status
- `/mcp exec <tool> <args>` - Execute MCP tool

---

## 3. A2A Integration (Agent-to-Agent)

### Concept
A2A enables carik-bot to communicate with other AI agents and delegate tasks.

### A2A Protocol
- JSON-RPC based messaging
- Task delegation with results
- Agent discovery
- State sharing

### Implementation

```rust
// A2A Client
struct A2AClient {
    endpoint: String,
    agent_id: String,
    capabilities: Vec<String>,
}

struct A2AMessage {
    id: String,
    method: String,
    params: Value,
    result: Option<Value>,
}

// Send task to another agent
async fn delegate_to_agent(
    agent_id: &str, 
    task: &str, 
    context: Value
) -> Result<String, String> {
    let client = get_a2a_client(agent_id).await?;
    
    let request = A2AMessage {
        id: uuid::new_v4().to_string(),
        method: "tasks/execute".to_string(),
        params: json!({
            "task": task,
            "context": context,
        }),
        result: None,
    };
    
    client.send(request).await
}
```

### Config
```yaml
a2a:
  enabled: true
  port: 8080
  agents:
    kiro:
      endpoint: http://localhost:8081
      capabilities: ["coding", "code-review"]
    claude:
      endpoint: http://localhost:8082
      capabilities: ["analysis", "writing"]
```

### Commands
- `/a2a list` - List connected agents
- `/a2a delegate <agent> <task>` - Delegate task
- `/a2a status` - Check agent status

### Use Cases
1. **Coding** → Delegate to Kiro
2. **Analysis** → Delegate to Claude
3. **Research** → Delegate to specialized agents

---

## 4. Wasm Plugins

### Concept
Wasm plugins allow users to write custom plugins in any language that compiles to WebAssembly (Rust, C, C++, Go, etc.)

### Wasm Plugin Structure

```rust
// Wasm plugin interface
#[wasm_bindgen]
pub trait CarikPlugin {
    fn name(&self) -> String;
    fn execute(&self, input: &str) -> String;
    fn cleanup(&self);
}

// Plugin manifest (plugin.json)
{
    "name": "weather-plugin",
    "version": "1.0.0",
    "author": "user",
    "description": "Get weather information",
    "entry": "weather.wasm",
    "config": {
        "api_key": "optional"
    }
}
```

### Plugin Manager

```rust
struct WasmPluginManager {
    plugins: HashMap<String, WasmModule>,
}

impl WasmPluginManager {
    fn load_plugin(&mut self, path: &str) -> Result<(), String> {
        let bytes = std::fs::read(path)?;
        let module = wasmtime::Module::new(&self.engine, &bytes)?;
        self.plugins.insert(module.name().to_string(), module);
        Ok(())
    }
    
    fn execute(&self, name: &str, input: &str) -> Result<String, String> {
        let instance = self.instantiate(name)?;
        instance.execute(input)
    }
}
```

### Config
```yaml
wasm:
  enabled: true
  plugins_dir: /home/ubuntu/.carik-bot/wasm-plugins
  sandbox_memory_limit: 128MB
  sandbox_cpu_limit: 1.0
```

### Commands
- `/wasm list` - List loaded plugins
- `/wasm load <name>` - Load a plugin
- `/wasm unload <name>` - Unload a plugin
- `/wasm <plugin> <args>` - Execute plugin

---

## 5. Unified Plugin Interface

### Plugin Registry

```rust
enum PluginType {
    Mcp(McpPlugin),
    A2A(A2AAgent),
    Wasm(WasmPlugin),
}

struct PluginRegistry {
    plugins: HashMap<String, PluginType>,
}

impl PluginRegistry {
    fn register(&mut self, plugin: PluginType) {
        let name = match &plugin {
            PluginType::Mcp(p) => p.name(),
            PluginType::A2A(p) => p.name(),
            PluginType::Wasm(p) => p.name(),
        };
        self.plugins.insert(name.to_string(), plugin);
    }
    
    fn execute(&self, name: &str, args: Value) -> Result<Value, String> {
        match self.plugins.get(name) {
            Some(PluginType::Mcp(p)) => p.execute(args),
            Some(PluginType::A2A(p)) => p.execute(args).await,
            Some(PluginType::Wasm(p)) => p.execute(args),
            None => Err(format!("Plugin '{}' not found", name)),
        }
    }
}
```

### Intent Detection for Plugin Routing

```rust
fn detect_plugin_intent(text: &str) -> Option<(String, Value)> {
    // Check for plugin commands
    if text.starts_with("/mcp ") || text.starts_with("/a2a ") || text.starts_with("/wasm ") {
        return None; // Direct command, not intent
    }
    
    // Use LLM or keyword matching to detect intent
    let keywords = [
        ("weather", "weather", "{}"),
        ("github", "github", r#"{"query": "{}"}"#),
        ("file", "filesystem", r#"{"path": "{}"}"#),
    ];
    
    for (keyword, plugin, template) in keywords {
        if text.to_lowercase().contains(keyword) {
            let args = template.replace("{}", &text);
            return Some((plugin.to_string(), serde_json::from_str(&args).unwrap()));
        }
    }
    
    None
}
```

---

## 6. Implementation Phases

### Phase 1: Foundation
- [ ] Create plugin trait/interface
- [ ] Build PluginManager
- [ ] Add config schema for plugins

### Phase 2: MCP Integration
- [ ] Implement MCP client
- [ ] Add filesystem MCP server
- [ ] Add HTTP MCP server
- [ ] Test with existing skills

### Phase 3: A2A Integration
- [ ] Implement A2A server (receive tasks)
- [ ] Implement A2A client (send tasks)
- [ ] Add Kiro as A2A agent
- [ ] Task delegation logic

### Phase 4: Wasm Integration
- [ ] Integrate wasmtime runtime
- [ ] Build plugin loader
- [ ] Add sandboxing (memory/CPU limits)
- [ ] Create example plugins

### Phase 5: Integration
- [ ] Update intent detection
- [ ] Add plugin routing
- [ ] Commands for management
- [ ] Documentation

---

## 7. File Structure

```
carik-bot/
├── src/
│   ├── plugins/
│   │   ├── mod.rs
│   │   ├── manager.rs       # Plugin registry & lifecycle
│   │   ├── trait.rs         # Plugin trait definition
│   │   ├── mcp/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs    # MCP client
│   │   │   └── servers/     # Built-in MCP servers
│   │   ├── a2a/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs    # A2A client
│   │   │   └── server.rs    # A2A server
│   │   └── wasm/
│   │       ├── mod.rs
│   │       ├── runtime.rs   # Wasmtime integration
│   │       └── loader.rs    # Plugin loader
│   └── main.rs
├── plugins/                 # Plugin directory
│   ├── mcp/               # MCP server configs
│   ├── wasm/              # Wasm plugins
│   └── a2a/               # A2A agent configs
├── config.yaml             # Plugin configuration
└── PLUGINS.md             # Plugin documentation
```

---

## 8. Security Considerations

1. **Sandboxing**: Wasm plugins run in isolated environment
2. **Rate Limiting**: Per-user plugin execution limits
3. **Access Control**: RBAC for sensitive plugins
4. **Audit Logging**: Log all plugin executions
5. **Timeout**: Plugin execution timeout (30s default)

---

## 9. Migration Path for Existing Skills

Convert OpenClaw skills to MCP plugins:

```rust
// Example: Convert weather skill to MCP
struct WeatherMcpServer;

impl McpServer for WeatherMcpServer {
    fn name(&self) -> &str { "weather" }
    
    fn tools(&self) -> Vec<McpTool> {
        vec![McpTool {
            name: "get_weather".to_string(),
            description: "Get current weather".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }),
        }]
    }
    
    async fn execute(&self, tool: &str, input: Value) -> Result<Value, String> {
        match tool {
            "get_weather" => {
                let location = input["location"].as_str().unwrap();
                get_weather(location).await
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
```
