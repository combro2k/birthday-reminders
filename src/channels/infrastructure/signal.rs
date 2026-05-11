use async_trait::async_trait;

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::SignalConfig;
use crate::reminders::domain::reminder::PendingReminder;

pub struct SignalSender {
    config: SignalConfig,
    signal_cli_path: String,
}

impl SignalSender {
    pub fn new(config: SignalConfig, signal_cli_path: String) -> Self {
        Self {
            config,
            signal_cli_path,
        }
    }

    fn run_send_command(&self, message: &str) -> Result<(), NotificationError> {
        use std::process::Command;

        let output = Command::new(&self.signal_cli_path)
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
                self.signal_cli_path,
                String::from_utf8_lossy(&output.stderr)
            ))),
            Err(e) => Err(NotificationError::SendFailed(format!(
                "Failed to execute {}: {}",
                self.signal_cli_path, e
            ))),
        }
    }
}

#[async_trait]
impl NotificationSender for SignalSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        let message = reminder.message();
        self.run_send_command(&message)
    }

    async fn test(&self) -> Result<(), NotificationError> {
        let test_message = "Test message from birthday-reminders";
        self.run_send_command(test_message)
    }
}
