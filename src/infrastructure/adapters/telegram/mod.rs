//! Telegram adapter

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::traits::{Bot, BotInfo, KeyboardButton};
use crate::application::errors::BotError;

/// Telegram API base URL
const API_BASE: &str = "https://api.telegram.org";

/// Telegram bot adapter
pub struct TelegramAdapter {
    token: String,
    client: Client,
    info: BotInfo,
}

impl TelegramAdapter {
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        Self {
            token: token.clone(),
            client: Client::new(),
            info: BotInfo {
                id: "unknown".to_string(),
                name: "carik-bot".to_string(),
                username: "carik_bot".to_string(),
            },
        }
    }

    /// Get the API URL for a method
    fn api_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", API_BASE, self.token, method)
    }

    /// Fetch bot info from Telegram API
    pub async fn fetch_bot_info(&mut self) -> Result<(), BotError> {
        #[derive(Deserialize)]
        struct Response {
            result: BotInfoResponse,
        }

        #[derive(Deserialize)]
        struct BotInfoResponse {
            id: i64,
            first_name: String,
            username: String,
        }

        let url = self.api_url("getMe");
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| BotError::Network(e.to_string()))?;

        let data: Response = response
            .json()
            .await
            .map_err(|e| BotError::Parse(e.to_string()))?;

        self.info = BotInfo {
            id: data.result.id.to_string(),
            name: data.result.first_name,
            username: data.result.username,
        };

        Ok(())
    }

    /// Send a message via Telegram API
    pub async fn send_message_api(&self, chat_id: &str, text: &str) -> Result<String, BotError> {
        #[derive(Serialize)]
        struct SendMessageRequest {
            chat_id: String,
            text: String,
        }

        #[derive(Deserialize)]
        struct Response {
            result: MessageResult,
        }

        #[derive(Deserialize)]
        struct MessageResult {
            message_id: i64,
        }

        let url = self.api_url("sendMessage");
        let request = SendMessageRequest {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BotError::Network(format!("Telegram API error: {}", response.status())));
        }

        let data: Response = response
            .json()
            .await
            .map_err(|e| BotError::Parse(e.to_string()))?;

        Ok(data.result.message_id.to_string())
    }
}

#[async_trait]
impl Bot for TelegramAdapter {
    async fn start(&self) -> Result<(), BotError> {
        tracing::info!("Starting Telegram bot (token: {}...)", &self.token[..8.min(self.token.len())]);
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, text: &str) -> Result<String, BotError> {
        tracing::debug!("Sending to {}: {}", chat_id, text);
        
        match self.send_message_api(chat_id, text).await {
            Ok(msg_id) => Ok(msg_id),
            Err(e) => {
                tracing::error!("Failed to send message: {}", e);
                Err(e)
            }
        }
    }

    async fn send_with_keyboard(&self, chat_id: &str, text: &str, buttons: Vec<Vec<KeyboardButton>>) -> Result<String, BotError> {
        tracing::debug!("Sending with keyboard to {}: {}", chat_id, text);
        
        // Build inline keyboard
        let inline_keyboard: Vec<Vec<InlineKeyboardButton>> = buttons.iter().map(|row| {
            row.iter().map(|btn| InlineKeyboardButton {
                text: btn.text.clone(),
                callback_data: btn.callback_data.clone(),
                url: btn.url.clone(),
            }).collect()
        }).collect();

        #[derive(Serialize)]
        struct SendMessageRequest {
            chat_id: String,
            text: String,
            reply_markup: ReplyMarkup,
        }

        #[derive(Serialize)]
        #[serde(untagged)]
        enum ReplyMarkup {
            Inline { inline_keyboard: Vec<Vec<InlineKeyboardButton>> },
        }

        #[derive(Serialize)]
        struct InlineKeyboardButton {
            text: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            callback_data: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            url: Option<String>,
        }

        let url = self.api_url("sendMessage");
        let request = SendMessageRequest {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            reply_markup: ReplyMarkup::Inline { inline_keyboard },
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BotError::Network(format!("Telegram API error: {}", response.status())));
        }

        #[derive(Deserialize)]
        struct Response {
            result: MessageResult,
        }

        #[derive(Deserialize)]
        struct MessageResult {
            message_id: i64,
        }

        let data: Response = response
            .json()
            .await
            .map_err(|e| BotError::Parse(e.to_string()))?;

        Ok(data.result.message_id.to_string())
    }

    async fn answer_callback(&self, callback_id: &str, text: Option<&str>) -> Result<(), BotError> {
        #[derive(Serialize)]
        struct AnswerRequest {
            callback_query_id: String,
            text: Option<String>,
        }

        let url = self.api_url("answerCallbackQuery");
        let request = AnswerRequest {
            callback_query_id: callback_id.to_string(),
            text: text.map(|s| s.to_string()),
        };

        let _response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::Network(e.to_string()))?;

        Ok(())
    }

    fn bot_info(&self) -> BotInfo {
        self.info.clone()
    }
}
