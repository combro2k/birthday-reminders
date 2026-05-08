use async_trait::async_trait;
use reqwest::Client;

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::{NtfyAuthType, NtfyConfig};
use crate::reminders::domain::reminder::PendingReminder;

pub struct NtfySender {
    config: NtfyConfig,
    client: Client,
}

impl NtfySender {
    pub fn new(config: NtfyConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn build_url(&self) -> String {
        let base = self.config.server_url.trim_end_matches('/');
        let topic = self.config.topic.trim();
        format!("{}/{}", base, topic)
    }

    fn create_request(&self, title: &str, body: &str, priority: u8) -> reqwest::RequestBuilder {
        let mut req = self
            .client
            .post(self.build_url())
            .header("Title", title)
            .header("Priority", priority.to_string())
            .body(body.to_string());

        // Add authentication headers if configured
        match self.config.auth_type {
            NtfyAuthType::None => {}
            NtfyAuthType::Basic => {
                if let (Some(username), Some(password)) =
                    (&self.config.username, &self.config.password)
                {
                    req = req.basic_auth(username, Some(password));
                }
            }
            NtfyAuthType::Bearer => {
                if let Some(token) = &self.config.token {
                    req = req.bearer_auth(token);
                }
            }
        }

        req
    }
}

#[async_trait]
impl NotificationSender for NtfySender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        let title = reminder.title();
        let message = reminder.message();
        let priority = self.config.priority_for_days_before(reminder.days_before);

        let response = self
            .create_request(&title, &message, priority)
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::SendFailed(format!(
                "Ntfy returned {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    async fn test(&self) -> Result<(), NotificationError> {
        let response = self
            .create_request(
                "Birthday Reminders - Test",
                "This is a test notification from Birthday Reminders.",
                self.config.priority_default,
            )
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::SendFailed(format!(
                "Ntfy test failed with {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}
