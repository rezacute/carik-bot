//! Telegram adapter

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::traits::{Bot, BotInfo, KeyboardButton};
use crate::application::errors::BotError;
use crate::infrastructure::config;

/// Telegram API base URL
const API_BASE: &str = "https://api.telegram.org";

/// Telegram update type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub message_id: i64,
    pub from: Option<User>,
    pub chat: Chat,
    pub text: Option<String>,
    pub reply_to_message: Option<Box<Message>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chat {
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    pub message: Option<Message>,
    pub data: Option<String>,
}

/// Telegram bot adapter
pub struct TelegramAdapter {
    token: String,
    client: Client,
    info: BotInfo,
    allowed_user_ids: Vec<String>,
}

impl TelegramAdapter {
    pub fn new(token: impl Into<String>, allowed_user_ids: Option<Vec<String>>) -> Self {
        let token = token.into();
        Self {
            token: token.clone(),
            client: Client::new(),
            info: BotInfo {
                id: "unknown".to_string(),
                name: "carik-bot".to_string(),
                username: "carik_bot".to_string(),
            },
            allowed_user_ids: allowed_user_ids.unwrap_or_default(),
        }
    }

    /// Check if user is whitelisted
    fn is_user_allowed(&self, user_id: &str) -> bool {
        if self.allowed_user_ids.is_empty() {
            true // No whitelist configured, allow all
        } else {
            self.allowed_user_ids.contains(&user_id.to_string())
        }
    }
    
    /// Add a user to the allowed list (e.g., when they use /connect)
    pub fn add_allowed_user(&mut self, user_id: String) {
        if !self.allowed_user_ids.contains(&user_id) {
            self.allowed_user_ids.push(user_id);
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

    /// Get updates from Telegram using getUpdates API
    pub async fn get_updates(&self, offset: i64, timeout: i64) -> Result<Vec<Update>, BotError> {
        #[derive(Serialize)]
        struct GetUpdatesRequest {
            offset: i64,
            timeout: i64,
            allowed_updates: Vec<String>,
        }

        #[derive(Deserialize)]
        struct Response {
            result: Vec<Update>,
        }

        let url = self.api_url("getUpdates");
        let request = GetUpdatesRequest {
            offset,
            timeout,
            allowed_updates: vec!["message".to_string(), "callback_query".to_string()],
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

        Ok(data.result)
    }

    /// Get the next update offset
    pub fn get_next_offset(updates: &[Update]) -> i64 {
        updates.iter()
            .map(|u| u.update_id + 1)
            .max()
            .unwrap_or(0)
    }

    /// Check if text has clear markdown formatting patterns
    fn has_markdown(text: &str) -> bool {
        // Only use markdown if clearly intended: **bold**, `code`, ```codeblocks```
        text.contains("**") || text.contains("`") || text.contains("```")
    }

    /// Escape special characters for Telegram MarkdownV2
    fn escape_markdown(text: &str) -> String {
        // Don't escape anything - send raw to Telegram
        // Telegram will render markdown if valid, show raw if not
        text.to_string()
    }

    /// Send a message via Telegram API - try MarkdownV2, fallback to plain
    pub async fn send_message_api(&self, chat_id: &str, text: &str) -> Result<String, BotError> {
        // Try with MarkdownV2 first
        match self.send_message_with_format(chat_id, text, Some("MarkdownV2")).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // Fallback to plain text
                tracing::warn!("Markdown failed, using plain text: {}", e);
                self.send_message_with_format(chat_id, text, None).await
            }
        }
    }

    /// Send a message with specific parse mode
    pub async fn send_message_with_format(&self, chat_id: &str, text: &str, parse_mode: Option<&str>) -> Result<String, BotError> {
        #[derive(Serialize)]
        struct SendMessageRequest {
            chat_id: String,
            text: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            parse_mode: Option<String>,
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
            parse_mode: parse_mode.map(|s| s.to_string()),
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

    /// Register bot commands with Telegram
    pub async fn register_commands(&self) -> Result<(), BotError> {
        #[derive(Serialize)]
        struct Command {
            command: String,
            description: String,
        }

        #[derive(Serialize)]
        struct SetMyCommandsRequest {
            commands: Vec<Command>,
        }

        let commands = vec![
            Command { command: "start".to_string(), description: "Start the bot".to_string() },
            Command { command: "help".to_string(), description: "Show help message".to_string() },
            Command { command: "version".to_string(), description: "Show bot version".to_string() },
            Command { command: "workspace".to_string(), description: "Manage workspaces".to_string() },
            Command { command: "code".to_string(), description: "Run coding task with kiro".to_string() },
            Command { command: "ping".to_string(), description: "Check bot is alive".to_string() },
            Command { command: "clear".to_string(), description: "Clear conversation".to_string() },
            Command { command: "quote".to_string(), description: "Get random quote".to_string() },
        ];

        let url = self.api_url("setMyCommands");
        let request = SetMyCommandsRequest { commands };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BotError::Network(format!("Failed to register commands: {}", error)));
        }

        tracing::info!("Registered bot commands with Telegram");
        Ok(())
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
        
        // Check whitelist - both in-memory and config file
        let in_memory_allowed = self.allowed_user_ids.is_empty() || self.is_user_allowed(chat_id);
        
        // Also check config file for dynamically added users
        let config_allowed = if !in_memory_allowed {
            if let Ok(config) = crate::infrastructure::config::Config::load("config.yaml") {
                config.whitelist.users.contains(&chat_id.to_string())
            } else {
                false
            }
        } else {
            true
        };
        
        if !in_memory_allowed && !config_allowed {
            tracing::warn!("Unauthorized user attempted to send message: {}", chat_id);
            return Err(BotError::Unauthorized("User not in whitelist".to_string()));
        }
        
        // Send typing action first
        let _ = self.send_chat_action(chat_id, "typing").await;
        
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

impl TelegramAdapter {
    /// Send chat action (typing, upload_photo, etc.)
    pub async fn send_chat_action(&self, chat_id: &str, action: &str) -> Result<(), BotError> {
        #[derive(Serialize)]
        struct SendChatActionRequest {
            chat_id: String,
            action: String,
        }
        
        let url = self.api_url("sendChatAction");
        let request = SendChatActionRequest {
            chat_id: chat_id.to_string(),
            action: action.to_string(),
        };
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::Network(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(BotError::Network(format!("Chat action error: {}", response.status())));
        }
        
        Ok(())
    }
}
