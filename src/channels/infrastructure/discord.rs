use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::DiscordConfig;
use crate::reminders::domain::reminder::PendingReminder;
use async_trait::async_trait;
use reqwest::Client;

pub struct DiscordSender {
    config: DiscordConfig,
    client: Client,
}

impl DiscordSender {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    async fn send_payload(&self, content: &str) -> Result<(), NotificationError> {
        let payload = serde_json::json!({ "content": content });

        let response = self
            .client
            .post(&self.config.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::SendFailed(format!(
                "Discord webhook returned {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl NotificationSender for DiscordSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        let content = format!("**{}**\n{}", reminder.title(), reminder.message());
        self.send_payload(&content).await
    }

    async fn test(&self) -> Result<(), NotificationError> {
        self.send_payload(
            "🎂 **Birthday Reminders** - Test notification. Your Discord configuration is working!",
        )
        .await
    }
}
