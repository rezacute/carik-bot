use super::User;
use chrono::{DateTime, Utc};

/// Type of message content
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Text,
    Command,
    Callback,
    Photo,
    Document,
    Audio,
    Video,
    Sticker,
    Location,
    Other(String),
}

impl MessageType {
    pub fn as_str(&self) -> &str {
        match self {
            MessageType::Text => "text",
            MessageType::Command => "command",
            MessageType::Callback => "callback",
            MessageType::Photo => "photo",
            MessageType::Document => "document",
            MessageType::Audio => "audio",
            MessageType::Video => "video",
            MessageType::Sticker => "sticker",
            MessageType::Location => "location",
            MessageType::Other(s) => s,
        }
    }
}

/// Message content
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Text(String),
    Command { name: String, args: Vec<String> },
    CallbackData(String),
    Empty,
}

impl Content {
    pub fn text(&self) -> Option<&str> {
        match self {
            Content::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_command(&self) -> bool {
        matches!(self, Content::Command { .. })
    }
}

/// Represents an incoming or outgoing message
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub chat_id: String,
    pub sender: Option<User>,
    pub content: Content,
    pub message_type: MessageType,
    pub timestamp: DateTime<Utc>,
    pub platform: String,
    pub raw: Option<serde_json::Value>,
}

impl Message {
    pub fn new(chat_id: impl Into<String>, content: Content) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            chat_id: chat_id.into(),
            sender: None,
            content,
            message_type: MessageType::Text,
            timestamp: Utc::now(),
            platform: "unknown".to_string(),
            raw: None,
        }
    }

    pub fn from_text(chat_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(chat_id, Content::Text(text.into()))
    }

    pub fn from_command(chat_id: impl Into<String>, name: impl Into<String>, args: Vec<String>) -> Self {
        let mut msg = Self::new(chat_id, Content::Command { name: name.into(), args });
        msg.message_type = MessageType::Command;
        msg
    }

    pub fn with_sender(mut self, user: User) -> Self {
        self.sender = Some(user);
        self
    }

    pub fn with_platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = platform.into();
        self
    }

    pub fn with_raw(mut self, raw: serde_json::Value) -> Self {
        self.raw = Some(raw);
        self
    }
}
