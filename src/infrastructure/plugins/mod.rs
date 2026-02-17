//! Plugin system for carik-bot
//! 
//! Plugins are dynamically loaded shared libraries that extend bot functionality.
//! Each plugin must implement the `Plugin` trait and provide a `plugin.toml` metadata file.

pub mod loader;
pub mod manifest;
pub mod registry;

pub use loader::PluginLoader;
pub use manifest::{PluginManifest, Permission};
pub use registry::PluginRegistry;
