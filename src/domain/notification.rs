use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::reminder::PendingReminder;

/// Supported notification channel types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Gotify,
    Email,
    Telegram,
    Signal,
    Whatsapp,
}

impl ChannelKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelKind::Gotify => "gotify",
            ChannelKind::Email => "email",
            ChannelKind::Telegram => "telegram",
            ChannelKind::Signal => "signal",
            ChannelKind::Whatsapp => "whatsapp",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "gotify" => Some(ChannelKind::Gotify),
            "email" => Some(ChannelKind::Email),
            "telegram" => Some(ChannelKind::Telegram),
            "signal" => Some(ChannelKind::Signal),
            "whatsapp" => Some(ChannelKind::Whatsapp),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ChannelKind::Gotify => "Gotify",
            ChannelKind::Email => "Email",
            ChannelKind::Telegram => "Telegram",
            ChannelKind::Signal => "Signal",
            ChannelKind::Whatsapp => "WhatsApp",
        }
    }

    pub fn all() -> &'static [ChannelKind] {
        &[
            ChannelKind::Gotify,
            ChannelKind::Email,
            ChannelKind::Telegram,
            ChannelKind::Signal,
            ChannelKind::Whatsapp,
        ]
    }

    /// Returns only channel kinds that have a working implementation
    pub fn implemented() -> &'static [ChannelKind] {
        &[
            ChannelKind::Gotify,
            ChannelKind::Email,
            ChannelKind::Telegram,
        ]
    }
}

/// Port trait for sending notifications
#[async_trait]
pub trait NotificationSender: Send + Sync {
    fn channel_kind(&self) -> ChannelKind;

    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError>;

    /// Send a test message to verify the channel is configured correctly
    async fn test(&self) -> Result<(), NotificationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Channel not configured")]
    NotConfigured,

    #[error("Channel not implemented: {0}")]
    NotImplemented(String),

    #[error("Failed to send notification: {0}")]
    SendFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
