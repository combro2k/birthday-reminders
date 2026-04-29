use std::sync::Arc;

use crate::domain::notification::ChannelKind;
use crate::domain::repository::{NotificationChannelRecord, NotificationChannelRepository, RepositoryError};
use crate::domain::user::UserId;
use crate::infrastructure::auth::crypto;
use crate::infrastructure::notifications::dispatcher;

pub struct NotificationCommandService {
    repo: Arc<dyn NotificationChannelRepository>,
    encryption_key: String,
}

impl NotificationCommandService {
    pub fn new(repo: Arc<dyn NotificationChannelRepository>, encryption_key: String) -> Self {
        Self { repo, encryption_key }
    }

    /// Decrypt config in a channel record
    fn decrypt_record(&self, mut record: NotificationChannelRecord) -> anyhow::Result<NotificationChannelRecord> {
        if let Some(encrypted) = record.config.get("_encrypted").and_then(|v| v.as_str()) {
            let decrypted_json = crypto::decrypt(encrypted, &self.encryption_key)?;
            record.config = serde_json::from_str(&decrypted_json)?;
        }
        Ok(record)
    }

    /// Encrypt a config value for storage
    fn encrypt_config(&self, config: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let json_str = serde_json::to_string(config)?;
        let encrypted = crypto::encrypt(&json_str, &self.encryption_key)?;
        Ok(serde_json::json!({ "_encrypted": encrypted }))
    }

    pub async fn list_channels(
        &self,
        user_id: &UserId,
    ) -> anyhow::Result<Vec<NotificationChannelRecord>> {
        let records = self.repo.find_for_user(user_id).await.map_err(|e| {
            anyhow::anyhow!("Failed to list channels: {}", e)
        })?;
        // Decrypt configs - skip records that fail decryption (log warning)
        Ok(records.into_iter().filter_map(|r| {
            match self.decrypt_record(r) {
                Ok(rec) => Some(rec),
                Err(e) => {
                    tracing::warn!("Failed to decrypt channel config: {}", e);
                    None
                }
            }
        }).collect())
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

        let encrypted_config = self.encrypt_config(&config)?;

        let record = self.repo
            .upsert(user_id, channel_type, enabled, encrypted_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to save channel: {}", e))?;

        // Return the record with decrypted config for display
        let mut decrypted = record;
        decrypted.config = config;
        Ok(decrypted)
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

        let decrypted = self.decrypt_record(record)?;

        let sender = dispatcher::build_sender(&decrypted)
            .map_err(|e| anyhow::anyhow!("Failed to build sender: {}", e))?;

        sender
            .test()
            .await
            .map_err(|e| anyhow::anyhow!("Test failed: {}", e))
    }
}
