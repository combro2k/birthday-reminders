use async_trait::async_trait;

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::WhatsappConfig;
use crate::reminders::domain::reminder::PendingReminder;

pub struct WhatsappSender {
    #[allow(dead_code)]
    config: WhatsappConfig,
}

impl WhatsappSender {
    pub fn new(config: WhatsappConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl NotificationSender for WhatsappSender {
    async fn send(&self, _reminder: &PendingReminder) -> Result<(), NotificationError> {
        Err(NotificationError::NotImplemented(
            "WhatsApp notifications are not yet implemented".to_string(),
        ))
    }

    async fn test(&self) -> Result<(), NotificationError> {
        Err(NotificationError::NotImplemented(
            "WhatsApp notifications are not yet implemented".to_string(),
        ))
    }
}
