use async_trait::async_trait;

use crate::domain::notification::{NotificationError, NotificationSender};
use crate::domain::notification_config::SignalConfig;
use crate::domain::reminder::PendingReminder;

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
    async fn send(&self, _reminder: &PendingReminder) -> Result<(), NotificationError> {
        Err(NotificationError::NotImplemented(
            "Signal notifications are not yet implemented".to_string(),
        ))
    }

    async fn test(&self) -> Result<(), NotificationError> {
        Err(NotificationError::NotImplemented(
            "Signal notifications are not yet implemented".to_string(),
        ))
    }
}
