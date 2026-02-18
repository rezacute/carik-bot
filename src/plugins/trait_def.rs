//! Plugin trait definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    /// Unique identifier for the plugin
    fn name(&self) -> &str;
    
    /// Human-readable description
    fn description(&self) -> &str;
    
    /// Execute the plugin with given arguments
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, String>;
    
    /// Optional: Cleanup resources when plugin is unloaded
    fn cleanup(&self) {}
    
    /// Optional: Get plugin metadata
    fn metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

/// Types of plugins supported
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum PluginKind {
    /// MCP (Model Context Protocol) plugin
    Mcp(McpPluginConfig),
    /// A2A (Agent-to-Agent) plugin
    A2A(A2APluginConfig),
    /// Wasm (WebAssembly) plugin
    Wasm(WasmPluginConfig),
}

/// Configuration for MCP plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPluginConfig {
    pub name: String,
    pub server_url: Option<String>,
    pub tools: Vec<String>,
}

/// Configuration for A2A agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2APluginConfig {
    pub name: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
}

/// Configuration for Wasm plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginConfig {
    pub name: String,
    pub path: String,
    pub config: Option<serde_json::Value>,
}

/// Extended plugin configuration for runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedPluginConfig {
    pub enabled: bool,
    pub plugins_dir: Option<String>,
    pub mcp: Option<HashMap<String, McpPluginConfig>>,
    pub a2a: Option<HashMap<String, A2APluginConfig>>,
    pub wasm: Option<HashMap<String, WasmPluginConfig>>,
}

impl Default for ExtendedPluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            plugins_dir: Some("/home/ubuntu/.carik-bot/plugins".to_string()),
            mcp: None,
            a2a: None,
            wasm: None,
        }
    }
}

/// Plugin execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl PluginResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }
    
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(msg.into()),
        }
    }
}
