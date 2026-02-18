//! Plugin system for carik-bot
//! 
//! Provides a unified interface for MCP, A2A, and Wasm plugins

pub mod manager;
pub mod trait_def;
pub mod mcp;
pub mod a2a;

pub use manager::PluginManager;
pub use trait_def::{Plugin, PluginKind, ExtendedPluginConfig as PluginConfig};
