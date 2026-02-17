use crate::domain::entities::Message;
use crate::application::errors::BotError;
use crate::domain::traits::Bot;

/// Service for processing messages
pub struct MessageService<B: Bot> {
    bot: B,
}

impl<B: Bot> MessageService<B> {
    pub fn new(bot: B) -> Self {
        Self { bot }
    }

    pub fn bot(&self) -> &B {
        &self.bot
    }

    /// Process an incoming message and return response
    pub async fn process(&self, message: Message) -> Result<Option<String>, BotError> {
        // Log incoming message
        tracing::info!("Processing message: {:?}", message.content);

        // Handle based on message type
        match message.content {
            crate::domain::entities::Content::Text(text) => {
                // Echo or process text
                Ok(Some(format!("Received: {}", text)))
            }
            crate::domain::entities::Content::Command { name, args } => {
                // Commands are handled by CommandService
                tracing::debug!("Command: {} with args: {:?}", name, args);
                Ok(Some(format!("Command: {}", name)))
            }
            crate::domain::entities::Content::CallbackData(data) => {
                // Handle callback
                tracing::debug!("Callback: {}", data);
                self.bot.answer_callback(&message.id, Some("Processing...")).await?;
                Ok(None)
            }
            crate::domain::entities::Content::Empty => Ok(None),
        }
    }

    /// Send a response message
    pub async fn respond(&self, chat_id: &str, text: &str) -> Result<String, BotError> {
        self.bot.send_message(chat_id, text).await
    }
}
