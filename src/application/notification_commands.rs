use std::sync::Arc;

use crate::domain::notification::ChannelKind;
use crate::domain::repository::{NotificationChannelRecord, NotificationChannelRepository, RepositoryError};
use crate::domain::user::UserId;
use crate::infrastructure::notifications::dispatcher;

pub struct NotificationCommandService {
    repo: Arc<dyn NotificationChannelRepository>,
}

impl NotificationCommandService {
    pub fn new(repo: Arc<dyn NotificationChannelRepository>) -> Self {
        Self { repo }
    }

    pub async fn list_channels(
        &self,
        user_id: &UserId,
    ) -> anyhow::Result<Vec<NotificationChannelRecord>> {
        self.repo.find_for_user(user_id).await.map_err(|e| {
            anyhow::anyhow!("Failed to list channels: {}", e)
        })
    }

    pub async fn upsert_channel(
        &self,
        user_id: &UserId,
        channel_type: &str,
        enabled: bool,
        config: serde_json::Value,
    ) -> anyhow::Result<NotificationChannelRecord> {
        // Validate channel type
        ChannelKind::from_str(channel_type)
            .ok_or_else(|| anyhow::anyhow!("Invalid channel type: {}", channel_type))?;

        self.repo
            .upsert(user_id, channel_type, enabled, config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to save channel: {}", e))
    }

    pub async fn delete_channel(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> anyhow::Result<()> {
        self.repo
            .delete(user_id, channel_type)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => anyhow::anyhow!("Channel not configured"),
                other => anyhow::anyhow!("Failed to delete channel: {}", other),
            })
    }

    pub async fn test_channel(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> anyhow::Result<()> {
        let record = self
            .repo
            .find_by_type(user_id, channel_type)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to find channel: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Channel not configured"))?;

        let sender = dispatcher::build_sender(&record)
            .map_err(|e| anyhow::anyhow!("Failed to build sender: {}", e))?;

        sender
            .test()
            .await
            .map_err(|e| anyhow::anyhow!("Test failed: {}", e))
    }
}
