//! MCP (Model Context Protocol) support
//! 
//! Phase 2 - Not yet implemented

use crate::plugins::trait_def::{McpPluginConfig, Plugin};

/// MCP plugin placeholder
pub struct McpPlugin {
    name: String,
    config: McpPluginConfig,
}

impl McpPlugin {
    pub fn new(name: impl Into<String>, config: McpPluginConfig) -> Self {
        Self {
            name: name.into(),
            config,
        }
    }
}

impl Plugin for McpPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "MCP plugin - external tool integration"
    }
    
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, String> {
        Err("MCP plugins not yet implemented (Phase 2)".to_string())
    }
}
