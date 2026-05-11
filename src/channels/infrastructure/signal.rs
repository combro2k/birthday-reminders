use async_trait::async_trait;
use serde_json::json;

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::SignalConfig;
use crate::reminders::domain::reminder::PendingReminder;

#[derive(Debug, Clone)]
pub enum SignalTransport {
    Cli { binary_path: String },
    Api { base_url: String },
}

#[derive(Debug, Clone)]
pub struct SignalRuntimeConfig {
    pub transport: SignalTransport,
}

pub struct SignalSender {
    config: SignalConfig,
    runtime: SignalRuntimeConfig,
}

impl SignalSender {
    pub fn new(config: SignalConfig, runtime: SignalRuntimeConfig) -> Self {
        Self { config, runtime }
    }

    fn send_via_cli(&self, binary_path: &str, message: &str) -> Result<(), NotificationError> {
        use std::process::Command;

        let output = Command::new(binary_path)
            .args([
                "-u",
                &self.config.sender,
                "send",
                "-m",
                message,
                &self.config.recipient,
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(NotificationError::SendFailed(format!(
                "{} failed: {}",
                binary_path,
                String::from_utf8_lossy(&output.stderr)
            ))),
            Err(e) => Err(NotificationError::SendFailed(format!(
                "Failed to execute {}: {}",
                binary_path, e
            ))),
        }
    }

    async fn send_via_api(&self, base_url: &str, message: &str) -> Result<(), NotificationError> {
        let endpoint = format!("{}/v2/send", base_url.trim_end_matches('/'));
        let payload = json!({
            "message": message,
            "number": self.config.sender,
            "recipients": [self.config.recipient],
        });

        let response = reqwest::Client::new()
            .post(&endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                NotificationError::SendFailed(format!("Signal API request failed: {}", e))
            })?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(NotificationError::SendFailed(format!(
            "Signal API returned {}: {}",
            status, body
        )))
    }

    async fn send_message(&self, message: &str) -> Result<(), NotificationError> {
        match &self.runtime.transport {
            SignalTransport::Cli { binary_path } => self.send_via_cli(binary_path, message),
            SignalTransport::Api { base_url } => self.send_via_api(base_url, message).await,
        }
    }
}

#[async_trait]
impl NotificationSender for SignalSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        let message = reminder.message();
        self.send_message(&message).await
    }

    async fn test(&self) -> Result<(), NotificationError> {
        let test_message = "Test message from birthday-reminders";
        self.send_message(test_message).await
    }
}
