//! LLM Configuration

use serde::{Deserialize, Serialize};

/// LLM Provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    MiniMax,
    Claude,
    Groq,
}

impl Default for LLMProvider {
    fn default() -> Self {
        Self::MiniMax
    }
}

/// LLM Configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct LLMConfig {
    /// Default provider
    pub provider: LLMProvider,
    
    /// Provider-specific API keys
    pub minimax_api_key: Option<String>,
    pub claude_api_key: Option<String>,
    pub groq_api_key: Option<String>,
    
    /// Default model for each provider
    pub minimax_model: Option<String>,
    pub claude_model: Option<String>,
    pub groq_model: Option<String>,
    
    /// Default settings
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::MiniMax,
            minimax_api_key: None,
            claude_api_key: None,
            groq_api_key: None,
            minimax_model: Some("abab6.5s-chat".to_string()),
            claude_model: Some("claude-3-haiku-20240307".to_string()),
            groq_model: Some("llama-3.1-70b-versatile".to_string()),
            temperature: 0.7,
            max_tokens: Some(1024),
            system_prompt: Some("You are carik, a helpful AI assistant.".to_string()),
        }
    }
}

impl LLMConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(key) = std::env::var("MINIMAX_API_KEY") {
            config.minimax_api_key = Some(key);
        }
        if let Ok(key) = std::env::var("CLAUDE_API_KEY") {
            config.claude_api_key = Some(key);
        }
        if let Ok(key) = std::env::var("GROQ_API_KEY") {
            config.groq_api_key = Some(key);
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            // Also check OPENAI_API_KEY for Groq compatibility
            if config.groq_api_key.is_none() {
                config.groq_api_key = Some(key);
            }
        }
        
        if let Ok(prompt) = std::env::var("LLM_SYSTEM_PROMPT") {
            config.system_prompt = Some(prompt);
        }
        
        if let Ok(temp) = std::env::var("LLM_TEMPERATURE") {
            if let Ok(t) = temp.parse() {
                config.temperature = t;
            }
        }
        
        config
    }
    
    /// Get API key for a provider
    pub fn api_key(&self, provider: LLMProvider) -> Option<&str> {
        match provider {
            LLMProvider::MiniMax => self.minimax_api_key.as_deref(),
            LLMProvider::Claude => self.claude_api_key.as_deref(),
            LLMProvider::Groq => self.groq_api_key.as_deref(),
        }
    }
    
    /// Get model for a provider
    pub fn model(&self, provider: LLMProvider) -> &str {
        match provider {
            LLMProvider::MiniMax => self.minimax_model.as_deref().unwrap_or("abab6.5s-chat"),
            LLMProvider::Claude => self.claude_model.as_deref().unwrap_or("claude-3-haiku-20240307"),
            LLMProvider::Groq => self.groq_model.as_deref().unwrap_or("llama-3.1-70b-versatile"),
        }
    }
}
