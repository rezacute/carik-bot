//! LLM integration - Multi-provider AI support

pub mod traits;
pub mod config;
pub mod providers;
#[cfg(test)]
pub mod tests;

pub use traits::{LLM, LLMMessage, LLMResponse, LLMError, LLMResult, LLMUsage};
pub use config::{LLMConfig, LLMProvider};
pub use providers::{MiniMaxProvider, ClaudeProvider, GroqProvider};
