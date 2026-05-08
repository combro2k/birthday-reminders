use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::reminders::domain::reminder::PendingReminder;

/// Broad category grouping for UI display
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelCategory {
    Email,
    Sms,
    Push,
    Messaging,
}

impl ChannelCategory {
    pub fn label(&self) -> &'static str {
        match self {
            ChannelCategory::Email => "Email",
            ChannelCategory::Sms => "SMS",
            ChannelCategory::Push => "Push Notifications",
            ChannelCategory::Messaging => "Messaging Apps",
        }
    }
}

/// Supported notification channel types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Gotify,
    Email,
    Telegram,
    Signal,
    Whatsapp,
    Discord,
    Sms,
    Ntfy,
    Pushover,
}

impl ChannelKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelKind::Gotify => "gotify",
            ChannelKind::Email => "email",
            ChannelKind::Telegram => "telegram",
            ChannelKind::Signal => "signal",
            ChannelKind::Whatsapp => "whatsapp",
            ChannelKind::Discord => "discord",
            ChannelKind::Sms => "sms",
            ChannelKind::Ntfy => "ntfy",
            ChannelKind::Pushover => "pushover",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "gotify" => Some(ChannelKind::Gotify),
            "email" => Some(ChannelKind::Email),
            "telegram" => Some(ChannelKind::Telegram),
            "signal" => Some(ChannelKind::Signal),
            "whatsapp" => Some(ChannelKind::Whatsapp),
            "discord" => Some(ChannelKind::Discord),
            "sms" => Some(ChannelKind::Sms),
            "ntfy" => Some(ChannelKind::Ntfy),
            "pushover" => Some(ChannelKind::Pushover),
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
            ChannelKind::Discord => "Discord",
            ChannelKind::Sms => "SMS (Twilio)",
            ChannelKind::Ntfy => "Ntfy",
            ChannelKind::Pushover => "Pushover",
        }
    }

    pub fn category(&self) -> ChannelCategory {
        match self {
            ChannelKind::Email => ChannelCategory::Email,
            ChannelKind::Sms => ChannelCategory::Sms,
            ChannelKind::Gotify | ChannelKind::Ntfy | ChannelKind::Pushover => {
                ChannelCategory::Push
            }
            ChannelKind::Telegram
            | ChannelKind::Signal
            | ChannelKind::Whatsapp
            | ChannelKind::Discord => ChannelCategory::Messaging,
        }
    }

    pub fn all() -> &'static [ChannelKind] {
        &[
            ChannelKind::Email,
            ChannelKind::Sms,
            ChannelKind::Gotify,
            ChannelKind::Ntfy,
            ChannelKind::Pushover,
            ChannelKind::Telegram,
            ChannelKind::Signal,
            ChannelKind::Whatsapp,
            ChannelKind::Discord,
        ]
    }

    /// Returns only channel kinds that have a working implementation
    pub fn implemented() -> &'static [ChannelKind] {
        &[
            ChannelKind::Gotify,
            ChannelKind::Email,
            ChannelKind::Telegram,
            ChannelKind::Whatsapp,
            ChannelKind::Discord,
            ChannelKind::Sms,
            ChannelKind::Ntfy,
            ChannelKind::Pushover,
        ]
    }
}

/// Port trait for sending notifications
#[async_trait]
pub trait NotificationSender: Send + Sync {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError>;

    /// Send a test message to verify the channel is configured correctly
    async fn test(&self) -> Result<(), NotificationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Failed to send notification: {0}")]
    SendFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
