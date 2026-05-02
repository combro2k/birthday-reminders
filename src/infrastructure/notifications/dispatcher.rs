use crate::domain::notification::{ChannelKind, NotificationError, NotificationSender};
use crate::domain::notification_config::*;
use crate::domain::repository::NotificationChannelRecord;

use super::discord::DiscordSender;
use super::email::EmailSender;
use super::gotify::GotifySender;
use super::signal::SignalSender;
use super::telegram::TelegramSender;
use super::whatsapp::WhatsappSender;

/// Build a NotificationSender from a channel record
pub fn build_sender(
    record: &NotificationChannelRecord,
) -> Result<Box<dyn NotificationSender>, NotificationError> {
    let kind = ChannelKind::from_str(&record.channel_type).ok_or_else(|| {
        NotificationError::InvalidConfig(format!("Unknown channel type: {}", record.channel_type))
    })?;

    match kind {
        ChannelKind::Gotify => {
            let config: GotifyConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(GotifySender::new(config)))
        }
        ChannelKind::Email => {
            let config: EmailConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(EmailSender::new(config)))
        }
        ChannelKind::Telegram => {
            let config: TelegramConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(TelegramSender::new(config)))
        }
        ChannelKind::Signal => {
            let config: SignalConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(SignalSender::new(config)))
        }
        ChannelKind::Whatsapp => {
            let config: WhatsappConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(WhatsappSender::new(config)))
        }
        ChannelKind::Discord => {
            let config: DiscordConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(DiscordSender::new(config)))
        }
    }
}
