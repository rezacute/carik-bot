//! Plugin manager - handles plugin lifecycle and execution

use crate::plugins::trait_def::{Plugin, ExtendedPluginConfig as PluginConfig, PluginResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn, error};

/// Manages all plugins for the bot
pub struct PluginManager {
    plugins: HashMap<String, Arc<dyn Plugin>>,
    config: PluginConfig,
}

impl PluginManager {
    /// Create a new plugin manager with config
    pub fn new(config: PluginConfig) -> Self {
        Self {
            plugins: HashMap::new(),
            config,
        }
    }
    
    /// Register a plugin
    pub fn register<P: Plugin + 'static>(&mut self, plugin: P) -> Result<(), String> {
        let name = plugin.name().to_string();
        
        if self.plugins.contains_key(&name) {
            return Err(format!("Plugin '{}' already registered", name));
        }
        
        info!("Registering plugin: {}", name);
        self.plugins.insert(name, Arc::new(plugin));
        Ok(())
    }
    
    /// Unregister a plugin
    pub fn unregister(&mut self, name: &str) -> Result<(), String> {
        if let Some(plugin) = self.plugins.remove(name) {
            plugin.cleanup();
            info!("Unregistered plugin: {}", name);
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", name))
        }
    }
    
    /// Execute a plugin by name
    pub fn execute(&self, name: &str, args: serde_json::Value) -> PluginResult {
        match self.plugins.get(name) {
            Some(plugin) => {
                match plugin.execute(args) {
                    Ok(result) => {
                        let output = serde_json::to_string(&result).unwrap_or_default();
                        PluginResult::success(output)
                    }
                    Err(e) => {
                        error!("Plugin '{}' error: {}", name, e);
                        PluginResult::error(e)
                    }
                }
            }
            None => {
                warn!("Plugin '{}' not found", name);
                PluginResult::error(format!("Plugin '{}' not found", name))
            }
        }
    }
    
    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(|(name, plugin)| {
            PluginInfo {
                name: name.clone(),
                description: plugin.description().to_string(),
                metadata: plugin.metadata(),
            }
        }).collect()
    }
    
    /// Check if a plugin exists
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
    
    /// Get config
    pub fn config(&self) -> &PluginConfig {
        &self.config
    }
    
    /// Load plugins from config
    pub fn load_from_config(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            info!("Plugin system disabled");
            return Ok(());
        }
        
        // Load MCP plugins
        if let Some(mcp_configs) = &self.config.mcp {
            for (name, config) in mcp_configs {
                info!("Loading MCP plugin: {}", name);
                // MCP plugins will be implemented in Phase 2
            }
        }
        
        // Load A2A plugins
        if let Some(a2a_configs) = &self.config.a2a {
            for (name, config) in a2a_configs {
                info!("Loading A2A agent: {}", name);
                // A2A plugins will be implemented in Phase 3
            }
        }
        
        // Load Wasm plugins
        if let Some(wasm_configs) = &self.config.wasm {
            for (name, config) in wasm_configs {
                info!("Loading Wasm plugin: {}", name);
                // Wasm plugins will be implemented in Phase 4
            }
        }
        
        Ok(())
    }
}

/// Plugin information for listing
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Thread-safe wrapper for PluginManager
pub type SharedPluginManager = Arc<RwLock<PluginManager>>;

/// Create a new shared plugin manager
pub fn create_plugin_manager(config: PluginConfig) -> SharedPluginManager {
    Arc::new(RwLock::new(PluginManager::new(config)))
}
