//! Plugin loader - Dynamically loads plugins from shared libraries

use std::path::{Path, PathBuf};
use std::sync::Arc;
use libloading::{Library, Symbol};
use crate::application::errors::{PluginError, PluginResult};
use super::manifest::PluginManifest;

/// Function signature for plugin initialization
pub type PluginInitFn = extern "C" fn() -> *mut dyn Plugin;

/// Main plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    /// Initialize the plugin
    fn init(&self) -> PluginResult<()>;
    
    /// Get plugin name
    fn name(&self) -> &str;
    
    /// Get plugin version
    fn version(&self) -> &str;
    
    /// Get plugin description
    fn description(&self) -> Option<&str>;
    
    /// Clean up resources when plugin is unloaded
    fn shutdown(&self) -> PluginResult<()>;
}

/// Loaded plugin instance
pub struct LoadedPlugin {
    #[allow(dead_code)]
    library: Library,
    manifest: PluginManifest,
    instance: Arc<dyn Plugin>,
}

impl LoadedPlugin {
    /// Get the plugin instance
    pub fn plugin(&self) -> &dyn Plugin {
        self.instance.as_ref()
    }
    
    /// Get the plugin instance as Arc
    pub fn into_instance(self) -> Arc<dyn Plugin> {
        self.instance
    }
    
    /// Get the plugin manifest
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
}

/// Plugin loader
pub struct PluginLoader {
    plugin_dir: PathBuf,
}

impl PluginLoader {
    pub fn new(plugin_dir: impl Into<PathBuf>) -> Self {
        Self {
            plugin_dir: plugin_dir.into(),
        }
    }

    /// Load a single plugin from a directory
    pub fn load_plugin(&self, path: impl AsRef<Path>) -> Result<LoadedPlugin, PluginError> {
        let path = path.as_ref();
        
        // Load manifest
        let manifest_path = path.join("plugin.toml");
        if !manifest_path.exists() {
            return Err(PluginError::Load(format!("Missing plugin.toml in {}", path.display())));
        }
        
        let manifest = PluginManifest::from_file(&manifest_path)?;
        
        // Resolve library path
        let library_path = if let Some(lib) = &manifest.library {
            path.join(lib)
        } else {
            // Default: look for libcarik_<name>.so
            path.join(format!("libcarik_{}.so", manifest.name))
        };
        
        if !library_path.exists() {
            return Err(PluginError::Load(format!("Library not found: {}", library_path.display())));
        }
        
        // Load the library
        let library = unsafe {
            Library::new(&library_path)
                .map_err(|e| PluginError::Load(format!("Failed to load library: {}", e)))?
        };
        
        // Get the init function
        let init_fn: Symbol<PluginInitFn> = unsafe {
            library.get(b"carik_plugin_init")
                .map_err(|e| PluginError::Load(format!("Failed to find init function: {}", e)))?
        };
        
        // Initialize the plugin
        let plugin = unsafe {
            let plugin_ptr = init_fn();
            if plugin_ptr.is_null() {
                return Err(PluginError::Load("Plugin init returned null".to_string()));
            }
            Arc::from_raw(plugin_ptr)
        };
        
        // Call init
        plugin.init()
            .map_err(|e| PluginError::Load(format!("Plugin init failed: {}", e)))?;
        
        tracing::info!("Loaded plugin: {} v{}", plugin.name(), plugin.version());
        
        Ok(LoadedPlugin {
            library,
            manifest,
            instance: plugin,
        })
    }

    /// Load all plugins from the plugin directory
    pub fn load_all(&self) -> Result<Vec<LoadedPlugin>, PluginError> {
        let mut plugins = Vec::new();
        
        if !self.plugin_dir.exists() {
            tracing::warn!("Plugin directory does not exist: {}", self.plugin_dir.display());
            return Ok(plugins);
        }
        
        for entry in std::fs::read_dir(&self.plugin_dir)
            .map_err(|e| PluginError::Load(format!("Failed to read plugin directory: {}", e)))? 
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };
            
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            
            // Skip hidden directories
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
            
            match self.load_plugin(&path) {
                Ok(plugin) => plugins.push(plugin),
                Err(e) => {
                    tracing::warn!("Failed to load plugin from {}: {}", path.display(), e);
                }
            }
        }
        
        Ok(plugins)
    }
}
