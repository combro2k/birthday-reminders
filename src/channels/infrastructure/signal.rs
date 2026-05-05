use async_trait::async_trait;

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::SignalConfig;
use crate::reminders::domain::reminder::PendingReminder;

pub struct SignalSender {
    #[allow(dead_code)]
    config: SignalConfig,
}

impl SignalSender {
    pub fn new(config: SignalConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl NotificationSender for SignalSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        use std::process::Command;

        // Use the configured sender (phone number) and recipient
        let sender = &self.config.api_url; // Actually the phone number registered with signal-cli
        let recipient = &self.config.recipient;
        let message = reminder.message();

        let output = Command::new("signal-cli")
            .args(["-u", sender, "send", "-m", &message, recipient])
            .output();

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(NotificationError::SendFailed(format!(
                "signal-cli failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))),
            Err(e) => Err(NotificationError::SendFailed(format!(
                "Failed to execute signal-cli: {}",
                e
            ))),
        }
    }

    async fn test(&self) -> Result<(), NotificationError> {
        use std::process::Command;
        let sender = &self.config.api_url;
        let recipient = &self.config.recipient;
        let test_message = "Test message from birthday-reminders";

        let output = Command::new("signal-cli")
            .args(["-u", sender, "send", "-m", test_message, recipient])
            .output();

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(NotificationError::SendFailed(format!(
                "signal-cli failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))),
            Err(e) => Err(NotificationError::SendFailed(format!(
                "Failed to execute signal-cli: {}",
                e
            ))),
        }
    }
}
