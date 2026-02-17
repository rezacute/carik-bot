//! Message parser - Parses raw messages into structured messages

use crate::domain::entities::{Message, Content, MessageType, User};

/// Parses incoming messages into structured Message objects
pub struct MessageParser {
    command_prefix: String,
}

impl MessageParser {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            command_prefix: prefix.into(),
        }
    }

    /// Parse a text message
    pub fn parse(&self, chat_id: impl Into<String>, text: impl Into<String>, sender: Option<User>) -> Message {
        let text = text.into();
        let chat_id = chat_id.into();
        
        // Check if it's a command
        if text.starts_with('/') || text.starts_with(&self.command_prefix) {
            return self.parse_command(chat_id, text, sender);
        }
        
        // Regular text message
        Message::new(chat_id, Content::Text(text))
            .with_message_type(MessageType::Text)
            .with_sender_opt(sender)
    }

    /// Parse a command message
    fn parse_command(&self, chat_id: String, text: String, sender: Option<User>) -> Message {
        // Remove the command prefix (either / or custom prefix)
        let cmd_text = if text.starts_with('/') {
            text.trim_start_matches('/')
        } else {
            text.trim_start_matches(&self.command_prefix)
        };
        
        // Split command and arguments
        let parts: Vec<&str> = cmd_text.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let args = parts.get(1..).map(|s| s.iter().map(|s| s.to_string()).collect());
        
        let content = if let Some(args) = args {
            Content::Command { name, args }
        } else {
            Content::Command { name, args: vec![] }
        };
        
        Message::new(chat_id, content)
            .with_message_type(MessageType::Command)
            .with_sender_opt(sender)
    }

    /// Parse a callback query (inline button press)
    pub fn parse_callback(&self, chat_id: impl Into<String>, data: impl Into<String>, user: User) -> Message {
        Message::new(chat_id, Content::CallbackData(data.into()))
            .with_message_type(MessageType::Callback)
            .with_sender(user)
    }
}

impl Message {
    /// Helper to set sender as Option
    pub fn with_sender_opt(mut self, user: Option<User>) -> Self {
        if let Some(u) = user {
            self.sender = Some(u);
        }
        self
    }

    /// Helper for MessageType
    pub fn with_message_type(mut self, mt: MessageType) -> Self {
        self.message_type = mt;
        self
    }
}
