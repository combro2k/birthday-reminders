use crate::domain::notification::{NotificationError, NotificationSender};
use crate::domain::notification_config::SmsConfig;
use crate::domain::reminder::PendingReminder;
use async_trait::async_trait;
use reqwest::Client;

pub struct SmsSender {
    config: SmsConfig,
    client: Client,
}

impl SmsSender {
    pub fn new(config: SmsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    async fn send_sms(&self, message: &str) -> Result<(), NotificationError> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.config.account_sid
        );

        let params = [
            ("To", &self.config.to_number),
            ("From", &self.config.from_number),
            ("Body", &message.to_string()),
        ];

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::SendFailed(format!(
                "Twilio API returned {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl NotificationSender for SmsSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        self.send_sms(&reminder.message()).await
    }

    async fn test(&self) -> Result<(), NotificationError> {
        self.send_sms(
            "🎂 Birthday Reminders - Test notification. Your SMS configuration is working!",
        )
        .await
    }
}
