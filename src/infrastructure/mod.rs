//! Infrastructure layer - External concerns
//! 
//! This layer contains:
//! - Config: Configuration loading
//! - Storage: Data persistence
//! - Adapters: Platform integrations (Telegram, Discord, etc.)
//! - Plugins: Plugin system
//! - LLM: AI integration

pub mod config;
pub mod database;
pub mod storage;
pub mod adapters;
pub mod plugins;
pub mod llm;
pub mod webcrawler;
pub mod financial;
