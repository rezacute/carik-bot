//! LLM integration - Multi-provider AI support

pub mod traits;
pub mod config;
pub mod providers;

pub use traits::{LLM, LLMMessage, LLMResponse, LLMError, LLMResult, LLMUsage};
pub use config::LLMConfig;
pub use providers::{MiniMaxProvider, ClaudeProvider, GroqProvider};
