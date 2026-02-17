//! File-based storage implementation

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::domain::traits::Store;
use crate::domain::entities::{User, Message};
use crate::application::errors::StorageError;

/// JSON file-based store
pub struct JsonStore {
    base_path: PathBuf,
    users: Arc<RwLock<HashMap<String, User>>>,
    messages: Arc<RwLock<HashMap<String, Vec<Message>>>>,
    kv: Arc<RwLock<HashMap<String, String>>>,
}

impl JsonStore {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            users: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
            kv: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn init(&self) -> Result<(), StorageError> {
        tokio::fs::create_dir_all(&self.base_path).await?;
        Ok(())
    }
}

#[async_trait]
impl Store for JsonStore {
    async fn get_user(&self, id: &str) -> Result<Option<User>, StorageError> {
        let users = self.users.read().await;
        Ok(users.get(id).cloned())
    }

    async fn save_user(&self, user: &User) -> Result<(), StorageError> {
        let mut users = self.users.write().await;
        users.insert(user.id.clone(), user.clone());
        Ok(())
    }

    async fn save_message(&self, message: &Message) -> Result<(), StorageError> {
        let mut messages = self.messages.write().await;
        messages.entry(message.chat_id.clone())
            .or_insert_with(Vec::new)
            .push(message.clone());
        Ok(())
    }

    async fn get_messages(&self, chat_id: &str, limit: usize) -> Result<Vec<Message>, StorageError> {
        let messages = self.messages.read().await;
        let chat_messages = messages.get(chat_id);
        
        match chat_messages {
            Some(msgs) => Ok(msgs.iter().rev().take(limit).cloned().collect()),
            None => Ok(Vec::new()),
        }
    }

    async fn get(&self, key: &str) -> Result<Option<String>, StorageError> {
        let kv = self.kv.read().await;
        Ok(kv.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), StorageError> {
        let mut kv = self.kv.write().await;
        kv.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let mut kv = self.kv.write().await;
        kv.remove(key);
        Ok(())
    }
}
