use async_trait::async_trait;
use uuid::Uuid;

use crate::infrastructure::error::RepositoryError;
use crate::users::domain::user::UserId;

/// Notification channel persistence
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NotificationChannelRecord {
    pub id: Uuid,
    pub user_id: UserId,
    pub channel_type: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait NotificationChannelRepository: Send + Sync {
    async fn find_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<NotificationChannelRecord>, RepositoryError>;

    async fn find_by_type(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<Option<NotificationChannelRecord>, RepositoryError>;

    async fn upsert(
        &self,
        user_id: &UserId,
        channel_type: &str,
        enabled: bool,
        config: serde_json::Value,
    ) -> Result<NotificationChannelRecord, RepositoryError>;

    async fn delete(&self, user_id: &UserId, channel_type: &str) -> Result<(), RepositoryError>;

    async fn find_enabled_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<NotificationChannelRecord>, RepositoryError>;
}
