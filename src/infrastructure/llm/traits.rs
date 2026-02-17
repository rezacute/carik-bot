//! LLM traits - Unified AI interface

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Chat message for LLM conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    /// Role: "system", "user", or "assistant"
    pub role: String,
    /// Message content
    pub content: String,
}

impl LLMMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
    
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
    
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    /// Response content
    pub content: String,
    /// Model used
    pub model: String,
    /// Number of tokens used (if available)
    pub usage: Option<LLMUsage>,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

/// LLM errors
#[derive(Debug)]
pub enum LLMError {
    /// API key missing
    MissingApiKey,
    /// Invalid request
    InvalidRequest(String),
    /// API error from provider
    ApiError(String),
    /// Network error
    NetworkError(String),
    /// Rate limited
    RateLimited,
    /// Parse error
    ParseError(String),
    /// Configuration error
    ConfigError(String),
}

impl std::fmt::Display for LLMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMError::MissingApiKey => write!(f, "Missing API key"),
            LLMError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            LLMError::ApiError(msg) => write!(f, "API error: {}", msg),
            LLMError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            LLMError::RateLimited => write!(f, "Rate limited"),
            LLMError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            LLMError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for LLMError {}

/// Result type for LLM operations
pub type LLMResult<T> = Result<T, LLMError>;

/// LLM Provider trait
#[async_trait]
pub trait LLM: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;
    
    /// Chat completion
    async fn chat(
        &self,
        messages: Vec<LLMMessage>,
        model: Option<&str>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> LLMResult<LLMResponse>;
    
    /// Streaming chat completion
    async fn chat_streaming(
        &self,
        messages: Vec<LLMMessage>,
        model: Option<&str>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> LLMResult<Box<dyn tokio::io::AsyncRead + Send + Unpin>>;
}
