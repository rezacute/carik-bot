//! Console adapter for development/testing

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::domain::traits::{Bot, BotInfo, KeyboardButton};
use crate::application::errors::BotError;

/// Console bot adapter for local development
pub struct ConsoleAdapter {
    info: BotInfo,
    sender: Option<mpsc::Sender<String>>,
}

impl ConsoleAdapter {
    pub fn new() -> Self {
        Self {
            info: BotInfo {
                id: "console".to_string(),
                name: "carik-bot".to_string(),
                username: "console".to_string(),
            },
            sender: None,
        }
    }

    pub fn with_sender(mut self, sender: mpsc::Sender<String>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub async fn read_line(&self, prompt: &str) -> Option<String> {
        print!("{}", prompt);
        use std::io::Read;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok()?;
        Some(input.trim().to_string())
    }
}

impl Default for ConsoleAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Bot for ConsoleAdapter {
    async fn start(&self) -> Result<(), BotError> {
        tracing::info!("Starting console bot (dev mode)");
        Ok(())
    }

    async fn send_message(&self, _chat_id: &str, text: &str) -> Result<String, BotError> {
        println!("[BOT] {}", text);
        Ok("console_msg".to_string())
    }

    async fn send_with_keyboard(&self, _chat_id: &str, text: &str, buttons: Vec<Vec<KeyboardButton>>) -> Result<String, BotError> {
        println!("[BOT] {}", text);
        for row in buttons {
            let row_text: Vec<String> = row.iter().map(|b| b.text.clone()).collect();
            println!("  [Buttons] {}", row_text.join(" | "));
        }
        Ok("console_msg".to_string())
    }

    async fn answer_callback(&self, _callback_id: &str, _text: Option<&str>) -> Result<(), BotError> {
        Ok(())
    }

    fn bot_info(&self) -> BotInfo {
        self.info.clone()
    }
}
