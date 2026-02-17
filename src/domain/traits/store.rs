use async_trait::async_trait;
use crate::application::errors::StorageError;

/// Store trait - abstraction for data persistence
#[async_trait]
pub trait Store: Send + Sync {
    // User operations
    async fn get_user(&self, id: &str) -> Result<Option<crate::domain::entities::User>, StorageError>;
    async fn save_user(&self, user: &crate::domain::entities::User) -> Result<(), StorageError>;

    // Message operations
    async fn save_message(&self, message: &crate::domain::entities::Message) -> Result<(), StorageError>;
    async fn get_messages(&self, chat_id: &str, limit: usize) -> Result<Vec<crate::domain::entities::Message>, StorageError>;

    // Key-value operations
    async fn get(&self, key: &str) -> Result<Option<String>, StorageError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
}
