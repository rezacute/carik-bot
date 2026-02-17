//! Infrastructure layer - External concerns
//! 
//! This layer contains:
//! - Config: Configuration loading
//! - Storage: Data persistence
//! - Adapters: Platform integrations (Telegram, Discord, etc.)
//! - Plugins: Plugin system

pub mod config;
pub mod storage;
pub mod adapters;
pub mod plugins;
