//! Plugin manifest definition

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Plugin metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PluginManifest {
    /// Plugin name (required)
    pub name: String,
    
    /// Plugin version (required)
    pub version: String,
    
    /// Plugin description
    pub description: Option<String>,
    
    /// Plugin author
    pub author: Option<String>,
    
    /// Path to the shared library
    pub library: Option<PathBuf>,
    
    /// Required permissions
    pub permissions: Vec<Permission>,
    
    /// Plugin dependencies
    pub dependencies: Vec<String>,
    
    /// Minimum carik-bot version required
    pub min_bot_version: Option<String>,
}

impl PluginManifest {
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, crate::application::errors::PluginError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::application::errors::PluginError::Load(format!("Failed to read manifest: {}", e)))?;
        
        serde_yaml::from_str(&content)
            .map_err(|e| crate::application::errors::PluginError::Load(format!("Failed to parse manifest: {}", e)))
    }
}

/// Permission types that a plugin can request
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
    /// Read messages
    ReadMessages,
    /// Send messages
    SendMessages,
    /// Manage commands
    ManageCommands,
    /// Access file system
    FileSystem,
    /// Make HTTP requests
    Http,
    /// Access environment variables
    EnvVars,
    /// Load other plugins
    LoadPlugins,
}

impl Permission {
    pub fn as_str(&self) -> &str {
        match self {
            Permission::ReadMessages => "read-messages",
            Permission::SendMessages => "send-messages",
            Permission::ManageCommands => "manage-commands",
            Permission::FileSystem => "filesystem",
            Permission::Http => "http",
            Permission::EnvVars => "env-vars",
            Permission::LoadPlugins => "load-plugins",
        }
    }
}
