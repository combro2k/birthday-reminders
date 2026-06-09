use crate::channels::domain::notification::{ChannelKind, NotificationError, NotificationSender};
use crate::channels::domain::notification_config::*;

use crate::channels::domain::repository::NotificationChannelRecord;

use super::discord::DiscordSender;
use super::email::EmailSender;
use super::gotify::GotifySender;
use super::ntfy::NtfySender;
use super::pushover::PushoverSender;
use super::signal::{SignalRuntimeConfig, SignalSender};
use super::sms::SmsSender;
use super::telegram::TelegramSender;
use super::whatsapp::WhatsappSender;

/// Context for email-specific features (List-Unsubscribe headers).
pub struct EmailContext {
    pub unsubscribe_url: String,
    pub list_id_domain: String,
}

/// Build a NotificationSender from a channel record
pub fn build_sender(
    record: &NotificationChannelRecord,
    signal_runtime: &SignalRuntimeConfig,
) -> Result<Box<dyn NotificationSender>, NotificationError> {
    build_sender_with_email_ctx(record, signal_runtime, None)
}

/// Build a NotificationSender, optionally with email unsubscribe context.
pub fn build_sender_with_email_ctx(
    record: &NotificationChannelRecord,
    signal_runtime: &SignalRuntimeConfig,
    email_ctx: Option<&EmailContext>,
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
            let mut sender = EmailSender::new(config);
            if let Some(ctx) = email_ctx {
                sender = sender
                    .with_unsubscribe(ctx.unsubscribe_url.clone(), ctx.list_id_domain.clone());
            }
            Ok(Box::new(sender))
        }
        ChannelKind::Telegram => {
            let config: TelegramConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(TelegramSender::new(config)))
        }
        ChannelKind::Signal => {
            let config: SignalConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(SignalSender::new(config, signal_runtime.clone())))
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
        ChannelKind::Sms => {
            let config: SmsConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(SmsSender::new(config)))
        }
        ChannelKind::Ntfy => {
            let config: NtfyConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(NtfySender::new(config)))
        }
        ChannelKind::Pushover => {
            let config: PushoverConfig = serde_json::from_value(record.config.clone())
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?;
            Ok(Box::new(PushoverSender::new(config)))
        }
    }
}
