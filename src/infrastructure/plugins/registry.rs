//! Plugin registry - Manages loaded plugins

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::application::errors::PluginError;
use super::loader::{LoadedPlugin, Plugin};

/// Registry for managing loaded plugins
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a loaded plugin
    pub fn register(&self, plugin: LoadedPlugin) -> Result<(), PluginError> {
        let name = plugin.manifest().name.clone();
        let plugin_arc = plugin.into_instance();
        
        let mut plugins = self.plugins.write()
            .map_err(|_| PluginError::Internal("Lock poisoned".to_string()))?;
        
        if plugins.contains_key(&name) {
            return Err(PluginError::Load(format!("Plugin '{}' already loaded", name)));
        }
        
        plugins.insert(name, plugin_arc);
        Ok(())
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.read()
            .ok()?
            .get(name)
            .cloned()
    }

    /// Get all plugin names
    pub fn names(&self) -> Vec<String> {
        self.plugins.read()
            .ok()
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Check if a plugin is loaded
    pub fn is_loaded(&self, name: &str) -> bool {
        self.plugins.read()
            .ok()
            .map(|p| p.contains_key(name))
            .unwrap_or(false)
    }

    /// Unload a plugin
    pub fn unload(&self, name: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write()
            .map_err(|_| PluginError::Internal("Lock poisoned".to_string()))?;
        
        if plugins.remove(name).is_some() {
            tracing::info!("Unloaded plugin: {}", name);
            Ok(())
        } else {
            Err(PluginError::NotFound(name.to_string()))
        }
    }

    /// Get the number of loaded plugins
    pub fn len(&self) -> usize {
        self.plugins.read()
            .ok()
            .map(|p| p.len())
            .unwrap_or(0)
    }

    /// Check if no plugins are loaded
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
