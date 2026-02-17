//! MiniMax AI Provider

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::infrastructure::llm::{LLMMessage, LLMResponse, LLMError, LLMResult, LLM, LLMUsage};

/// MiniMax API endpoint
const API_BASE: &str = "https://api.minimax.chat/v1";

/// MiniMax provider
pub struct MiniMaxProvider {
    api_key: String,
    client: Client,
    model: String,
}

impl MiniMaxProvider {
    pub fn new(api_key: impl Into<String>, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            model: model.unwrap_or("abab6.5s-chat").to_string(),
        }
    }
    
    /// Get base URL for API
    fn base_url(&self) -> String {
        format!("{}/text/chatcompletion_v2", API_BASE)
    }
}

/// API request structure
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<LLMMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    stream: bool,
}

/// API response structure
#[derive(Deserialize, Debug)]
struct ChatResponse {
    id: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

/// Choice in response
#[derive(Deserialize, Debug)]
struct Choice {
    index: usize,
    message: ResponseMessage,
    finish_reason: Option<String>,
}

/// Response message
#[derive(Deserialize, Debug)]
struct ResponseMessage {
    role: String,
    content: String,
}

/// Usage information
#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[async_trait]
impl LLM for MiniMaxProvider {
    fn name(&self) -> &str {
        "minimax"
    }
    
    async fn chat(
        &self,
        messages: Vec<LLMMessage>,
        model: Option<&str>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> LLMResult<LLMResponse> {
        let model = model.unwrap_or(&self.model);
        
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens,
            stream: false,
        };
        
        let response = self.client
            .post(self.base_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(e.to_string()))?;
        
        if response.status() == 429 {
            return Err(LLMError::RateLimited);
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!("status: {}, body: {}", status, body)));
        }
        
        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;
        
        let choice = chat_response.choices
            .into_iter()
            .next()
            .ok_or_else(|| LLMError::InvalidRequest("No choices in response".to_string()))?;
        
        let usage = chat_response.usage.map(|u| LLMUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        
        Ok(LLMResponse {
            content: choice.message.content,
            model: model.to_string(),
            usage,
            finish_reason: choice.finish_reason,
        })
    }
    
    async fn chat_streaming(
        &self,
        _messages: Vec<LLMMessage>,
        _model: Option<&str>,
        _temperature: Option<f32>,
        _max_tokens: Option<u32>,
    ) -> LLMResult<Box<dyn tokio::io::AsyncRead + Send + Unpin>> {
        // TODO: Implement streaming
        Err(LLMError::InvalidRequest("Streaming not yet implemented".to_string()))
    }
}
