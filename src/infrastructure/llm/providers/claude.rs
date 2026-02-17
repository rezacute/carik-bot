//! Anthropic Claude Provider

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::infrastructure::llm::{LLMMessage, LLMResponse, LLMError, LLMResult, LLM, LLMUsage};

/// Claude API endpoint
const API_BASE: &str = "https://api.anthropic.com/v1";

/// Claude provider
pub struct ClaudeProvider {
    api_key: String,
    client: Client,
    model: String,
}

impl ClaudeProvider {
    pub fn new(api_key: impl Into<String>, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            model: model.unwrap_or("claude-3-haiku-20240307").to_string(),
        }
    }
    
    /// Get base URL for API
    fn base_url(&self) -> String {
        format!("{}/messages", API_BASE)
    }
}

/// API request structure
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

/// Claude message format
#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

impl From<&LLMMessage> for ClaudeMessage {
    fn from(msg: &LLMMessage) -> Self {
        Self {
            role: msg.role.clone(),
            content: msg.content.clone(),
        }
    }
}

/// API response structure
#[derive(Deserialize, Debug)]
struct ChatResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<ContentBlock>,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
    #[serde(rename = "stop_sequence")]
    stop_sequence: Option<String>,
    usage: Option<Usage>,
}

/// Content block
#[derive(Deserialize, Debug)]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

/// Usage information
#[derive(Deserialize, Debug)]
struct Usage {
    #[serde(rename = "input_tokens")]
    input_tokens: u32,
    #[serde(rename = "output_tokens")]
    output_tokens: u32,
}

#[async_trait]
impl LLM for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }
    
    async fn chat(
        &self,
        messages: Vec<LLMMessage>,
        model: Option<&str>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> LLMResult<LLMResponse> {
        let model = model.unwrap_or(&self.model);
        
        // Convert messages to Claude format
        let claude_messages: Vec<ClaudeMessage> = messages.iter().map(ClaudeMessage::from).collect();
        
        let request = ChatRequest {
            model: model.to_string(),
            messages: claude_messages,
            temperature,
            max_tokens,
        };
        
        let response = self.client
            .post(self.base_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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
        
        // Extract text content
        let content = chat_response.content
            .into_iter()
            .filter_map(|block| {
                if let ContentBlock::Text { text } = block {
                    Some(text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        let usage = chat_response.usage.map(|u| LLMUsage {
            prompt_tokens: Some(u.input_tokens),
            completion_tokens: Some(u.output_tokens),
            total_tokens: Some(u.input_tokens + u.output_tokens),
        });
        
        Ok(LLMResponse {
            content,
            model: model.to_string(),
            usage,
            finish_reason: chat_response.stop_reason,
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
