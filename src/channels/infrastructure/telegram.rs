use async_trait::async_trait;
use reqwest::Client;

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::TelegramConfig;
use crate::reminders::domain::reminder::PendingReminder;

pub struct TelegramSender {
    config: TelegramConfig,
    client: Client,
}

impl TelegramSender {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl NotificationSender for TelegramSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        let text = format!("{}\n\n{}", reminder.title(), reminder.message());
        self.send_message(&text).await
    }

    async fn test(&self) -> Result<(), NotificationError> {
        self.send_message(
            "🎂 Birthday Reminders - Test notification. Your Telegram configuration is working!",
        )
        .await
    }
}

impl TelegramSender {
    async fn send_message(&self, text: &str) -> Result<(), NotificationError> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.config.bot_token
        );

        let body = serde_json::json!({
            "chat_id": self.config.chat_id,
            "text": text,
            "parse_mode": "HTML",
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::SendFailed(format!(
                "Telegram API returned {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}
