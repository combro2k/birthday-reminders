use async_trait::async_trait;
use reqwest::Client;

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::PushoverConfig;
use crate::reminders::domain::reminder::PendingReminder;

const PUSHOVER_API_URL: &str = "https://api.pushover.net/1/messages.json";

pub struct PushoverSender {
    config: PushoverConfig,
    client: Client,
}

impl PushoverSender {
    pub fn new(config: PushoverConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    async fn post(&self, title: &str, message: &str) -> Result<(), NotificationError> {
        let response = self
            .client
            .post(PUSHOVER_API_URL)
            .form(&[
                ("token", self.config.api_token.as_str()),
                ("user", self.config.user_key.as_str()),
                ("title", title),
                ("message", message),
            ])
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::SendFailed(format!(
                "Pushover returned {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl NotificationSender for PushoverSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        self.post(&reminder.title(), &reminder.message()).await
    }

    async fn test(&self) -> Result<(), NotificationError> {
        self.post(
            "Birthday Reminders - Test",
            "This is a test notification from Birthday Reminders.",
        )
        .await
    }
}
