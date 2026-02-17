//! Telegram adapter

use async_trait::async_trait;
use crate::domain::traits::{Bot, BotInfo, KeyboardButton};
use crate::application::errors::BotError;

/// Telegram bot adapter
pub struct TelegramAdapter {
    token: String,
    info: BotInfo,
}

impl TelegramAdapter {
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        Self {
            info: BotInfo {
                id: "unknown".to_string(),
                name: "carik-bot".to_string(),
                username: "carik_bot".to_string(),
            },
            token,
        }
    }

    pub async fn fetch_bot_info(&mut self) -> Result<(), BotError> {
        // TODO: Implement API call to get bot info
        Ok(())
    }
}

#[async_trait]
impl Bot for TelegramAdapter {
    async fn start(&self) -> Result<(), BotError> {
        tracing::info!("Starting Telegram bot: {}", self.info.username);
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, text: &str) -> Result<String, BotError> {
        tracing::debug!("Sending to {}: {}", chat_id, text);
        // TODO: Implement actual Telegram API call
        Ok("message_id".to_string())
    }

    async fn send_with_keyboard(&self, chat_id: &str, text: &str, _buttons: Vec<Vec<KeyboardButton>>) -> Result<String, BotError> {
        tracing::debug!("Sending with keyboard to {}: {}", chat_id, text);
        Ok("message_id".to_string())
    }

    async fn answer_callback(&self, callback_id: &str, _text: Option<&str>) -> Result<(), BotError> {
        tracing::debug!("Answering callback: {}", callback_id);
        Ok(())
    }

    fn bot_info(&self) -> BotInfo {
        self.info.clone()
    }
}
