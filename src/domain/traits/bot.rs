use async_trait::async_trait;
use crate::domain::entities::Message;
use crate::application::errors::BotError;

/// Bot trait - abstraction for messaging platform adapters
#[async_trait]
pub trait Bot: Send + Sync {
    /// Start the bot and begin listening for messages
    async fn start(&self) -> Result<(), BotError>;

    /// Send a message to a chat
    async fn send_message(&self, chat_id: &str, text: &str) -> Result<String, BotError>;

    /// Send a message with inline keyboard
    async fn send_with_keyboard(&self, chat_id: &str, text: &str, buttons: Vec<Vec<KeyboardButton>>) -> Result<String, BotError>;

    /// Answer a callback query
    async fn answer_callback(&self, callback_id: &str, text: Option<&str>) -> Result<(), BotError>;

    /// Get bot info
    fn bot_info(&self) -> BotInfo;
}

/// Keyboard button for inline keyboards
#[derive(Debug, Clone)]
pub struct KeyboardButton {
    pub text: String,
    pub callback_data: Option<String>,
    pub url: Option<String>,
}

impl KeyboardButton {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            callback_data: None,
            url: None,
        }
    }

    pub fn with_callback(mut self, data: impl Into<String>) -> Self {
        self.callback_data = Some(data.into());
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

/// Bot information
#[derive(Debug, Clone)]
pub struct BotInfo {
    pub id: String,
    pub name: String,
    pub username: String,
}
