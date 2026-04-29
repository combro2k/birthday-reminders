use async_trait::async_trait;
use reqwest::Client;

use crate::domain::notification::{ChannelKind, NotificationError, NotificationSender};
use crate::domain::notification_config::GotifyConfig;
use crate::domain::reminder::PendingReminder;

pub struct GotifySender {
    config: GotifyConfig,
    client: Client,
}

impl GotifySender {
    pub fn new(config: GotifyConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl NotificationSender for GotifySender {
    fn channel_kind(&self) -> ChannelKind {
        ChannelKind::Gotify
    }

    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        let url = format!("{}/message", self.config.url.trim_end_matches('/'));

        let body = serde_json::json!({
            "title": reminder.title(),
            "message": reminder.message(),
            "priority": 5,
        });

        let response = self
            .client
            .post(&url)
            .header("X-Gotify-Key", &self.config.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::SendFailed(format!(
                "Gotify returned {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    async fn test(&self) -> Result<(), NotificationError> {
        let url = format!("{}/message", self.config.url.trim_end_matches('/'));

        let body = serde_json::json!({
            "title": "Birthday Reminders - Test",
            "message": "This is a test notification from Birthday Reminders.",
            "priority": 5,
        });

        let response = self
            .client
            .post(&url)
            .header("X-Gotify-Key", &self.config.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::SendFailed(format!(
                "Gotify returned {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}
