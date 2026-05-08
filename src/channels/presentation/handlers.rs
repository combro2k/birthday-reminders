use std::sync::Arc;

use axum::{
    Form,
    extract::{Extension, Path, Query, State},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::auth::infrastructure::session::get_csrf_token;
use crate::channels::domain::notification::ChannelKind;
use crate::channels::domain::notification_config::{
    DiscordConfig, EmailConfig, EmailProvider, GotifyConfig, NtfyAuthType, NtfyConfig,
    PushoverConfig, SignalConfig, SmsConfig, SmtpSecurity, TelegramConfig, WhatsappConfig,
};
use crate::channels::domain::repository::NotificationChannelRecord;
use crate::channels::presentation::templates::{
    ChannelFormTemplate, ChannelKindView, ChannelsTemplate,
};
use crate::infrastructure::web::server::AppState;
use crate::users::domain::user::User;

#[derive(Debug, Deserialize, Default)]
pub struct TestResultQuery {
    pub test_ok: Option<String>,
    pub test_err: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TestChannelForm {
    pub source: Option<String>,
}

pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Query(params): Query<TestResultQuery>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let records = state
        .notification_service
        .list_channels(&user.id)
        .await
        .unwrap_or_default();

    let record_map: std::collections::HashMap<String, &NotificationChannelRecord> = records
        .iter()
        .map(|r| (r.channel_type.clone(), r))
        .collect();

    let available: Vec<ChannelKindView> = ChannelKind::implemented()
        .iter()
        .map(|k| {
            let rec = record_map.get(k.as_str());
            ChannelKindView {
                kind: k.as_str().to_string(),
                display_name: k.display_name().to_string(),
                configured: rec.is_some(),
                enabled: rec.map(|r| r.enabled),
            }
        })
        .collect();

    let template = ChannelsTemplate {
        user,
        available,
        csrf_token,
        test_success: params.test_ok,
        test_error: params.test_err,
    };
    Html(template.to_string())
}

pub async fn channel_form(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Path(channel_type): Path<String>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let kind = match ChannelKind::from_str(&channel_type) {
        Some(k) => k,
        None => return Redirect::to("/notifications").into_response(),
    };

    let existing = state
        .notification_service
        .list_channels(&user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.channel_type == channel_type);

    let template = channel_form_template(
        user,
        channel_type.clone(),
        kind,
        existing.as_ref(),
        existing.as_ref().map(|r| r.enabled).unwrap_or(true),
        ChannelConfigForm::default(),
        existing.is_some(),
        None,
        None,
        csrf_token,
    );
    Html(template.to_string()).into_response()
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ChannelConfigForm {
    pub enabled: Option<String>,
    pub gotify_url: Option<String>,
    pub gotify_token: Option<String>,
    pub email_provider: Option<String>,
    pub email_username: Option<String>,
    pub email_password: Option<String>,
    pub email_to: Option<String>,
    pub email_smtp_host: Option<String>,
    pub email_smtp_port: Option<String>,
    pub email_smtp_security: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub signal_api_url: Option<String>,
    pub signal_recipient: Option<String>,
    pub whatsapp_api_url: Option<String>,
    pub whatsapp_recipient: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub sms_account_sid: Option<String>,
    pub sms_auth_token: Option<String>,
    pub sms_from_number: Option<String>,
    pub sms_to_number: Option<String>,
    pub ntfy_server_url: Option<String>,
    pub ntfy_topic: Option<String>,
    pub ntfy_priority_default: Option<String>,
    pub ntfy_priority_today: Option<String>,
    pub ntfy_priority_tomorrow: Option<String>,
    pub ntfy_auth_type: Option<String>,
    pub ntfy_username: Option<String>,
    pub ntfy_password: Option<String>,
    pub ntfy_token: Option<String>,
    pub pushover_api_token: Option<String>,
    pub pushover_user_key: Option<String>,
}

fn trim_or_empty(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}

fn required_field(value: Option<&str>, field_name: &str) -> Result<String, String> {
    let trimmed = value.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        Err(format!("{} is required", field_name))
    } else {
        Ok(trimmed.to_string())
    }
}

fn parse_email_provider(value: Option<&str>) -> Result<EmailProvider, String> {
    match value.map(str::trim).unwrap_or("gmail") {
        "gmail" => Ok(EmailProvider::Gmail),
        "proton" => Ok(EmailProvider::Proton),
        "proton_smtp" => Ok(EmailProvider::ProtonSmtp),
        "outlook" => Ok(EmailProvider::Outlook),
        "custom" => Ok(EmailProvider::Custom),
        other => Err(format!("Invalid email provider: {}", other)),
    }
}

fn email_provider_as_str(provider: EmailProvider) -> &'static str {
    match provider {
        EmailProvider::Gmail => "gmail",
        EmailProvider::Proton => "proton",
        EmailProvider::ProtonSmtp => "proton_smtp",
        EmailProvider::Outlook => "outlook",
        EmailProvider::Custom => "custom",
    }
}

fn parse_smtp_security(value: Option<&str>) -> Result<SmtpSecurity, String> {
    match value.map(str::trim).unwrap_or("starttls") {
        "starttls" => Ok(SmtpSecurity::Starttls),
        "tls" => Ok(SmtpSecurity::Tls),
        "none" => Ok(SmtpSecurity::None),
        other => Err(format!("Invalid SMTP security value: {}", other)),
    }
}

fn smtp_security_as_str(security: SmtpSecurity) -> &'static str {
    match security {
        SmtpSecurity::Starttls => "starttls",
        SmtpSecurity::Tls => "tls",
        SmtpSecurity::None => "none",
    }
}

fn parse_ntfy_auth_type(value: Option<&str>) -> Result<NtfyAuthType, String> {
    match value.map(str::trim).unwrap_or("none") {
        "none" => Ok(NtfyAuthType::None),
        "basic" => Ok(NtfyAuthType::Basic),
        "bearer" => Ok(NtfyAuthType::Bearer),
        other => Err(format!("Invalid Ntfy auth type: {}", other)),
    }
}

fn ntfy_auth_type_as_str(auth_type: NtfyAuthType) -> &'static str {
    match auth_type {
        NtfyAuthType::None => "none",
        NtfyAuthType::Basic => "basic",
        NtfyAuthType::Bearer => "bearer",
    }
}

fn parse_ntfy_priority(value: Option<&str>, field_name: &str) -> Result<Option<u8>, String> {
    let trimmed = value.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let priority = trimmed
        .parse::<u8>()
        .map_err(|_| format!("{} must be a number between 1 and 5", field_name))?;

    if (1..=5).contains(&priority) {
        Ok(Some(priority))
    } else {
        Err(format!("{} must be between 1 and 5", field_name))
    }
}

fn build_config(kind: ChannelKind, form: &ChannelConfigForm) -> Result<serde_json::Value, String> {
    match kind {
        ChannelKind::Gotify => {
            let cfg = GotifyConfig {
                url: required_field(form.gotify_url.as_deref(), "Gotify URL")?,
                token: required_field(form.gotify_token.as_deref(), "Gotify token")?,
            };
            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
        ChannelKind::Email => {
            let provider = parse_email_provider(form.email_provider.as_deref())?;
            let mut cfg = EmailConfig {
                provider,
                username: required_field(form.email_username.as_deref(), "SMTP username")?,
                password: required_field(form.email_password.as_deref(), "SMTP password")?,
                to: required_field(form.email_to.as_deref(), "Recipient email")?,
                smtp_host: None,
                smtp_port: None,
                security: None,
            };

            if provider == EmailProvider::Custom {
                cfg.smtp_host = Some(required_field(
                    form.email_smtp_host.as_deref(),
                    "Custom SMTP host",
                )?);

                let port_raw = required_field(form.email_smtp_port.as_deref(), "Custom SMTP port")?;
                let port = port_raw.parse::<u16>().map_err(|_| {
                    "Custom SMTP port must be a number between 1 and 65535".to_string()
                })?;
                cfg.smtp_port = Some(port);
                cfg.security = Some(parse_smtp_security(form.email_smtp_security.as_deref())?);
            }

            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
        ChannelKind::Telegram => {
            let cfg = TelegramConfig {
                bot_token: required_field(
                    form.telegram_bot_token.as_deref(),
                    "Telegram bot token",
                )?,
                chat_id: required_field(form.telegram_chat_id.as_deref(), "Telegram chat ID")?,
            };
            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
        ChannelKind::Signal => {
            let cfg = SignalConfig {
                api_url: required_field(form.signal_api_url.as_deref(), "Signal API URL")?,
                recipient: required_field(form.signal_recipient.as_deref(), "Signal recipient")?,
            };
            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
        ChannelKind::Whatsapp => {
            let cfg = WhatsappConfig {
                api_url: required_field(form.whatsapp_api_url.as_deref(), "WhatsApp API URL")?,
                recipient: required_field(
                    form.whatsapp_recipient.as_deref(),
                    "WhatsApp recipient",
                )?,
            };
            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
        ChannelKind::Discord => {
            let cfg = DiscordConfig {
                webhook_url: required_field(
                    form.discord_webhook_url.as_deref(),
                    "Discord webhook URL",
                )?,
            };
            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
        ChannelKind::Sms => {
            let cfg = SmsConfig {
                account_sid: required_field(form.sms_account_sid.as_deref(), "Account SID")?,
                auth_token: required_field(form.sms_auth_token.as_deref(), "Auth Token")?,
                from_number: required_field(form.sms_from_number.as_deref(), "From Number")?,
                to_number: required_field(form.sms_to_number.as_deref(), "To Number")?,
            };
            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
        ChannelKind::Ntfy => {
            let server_url = trim_or_empty(form.ntfy_server_url.as_deref());
            let server_url = if server_url.is_empty() {
                "https://ntfy.sh".to_string()
            } else {
                server_url
            };

            let auth_type = parse_ntfy_auth_type(form.ntfy_auth_type.as_deref())?;
            let priority_default =
                parse_ntfy_priority(form.ntfy_priority_default.as_deref(), "Default priority")?
                    .unwrap_or(3);
            let cfg = NtfyConfig {
                server_url,
                topic: required_field(form.ntfy_topic.as_deref(), "Ntfy topic")?,
                priority_default,
                priority_today: parse_ntfy_priority(
                    form.ntfy_priority_today.as_deref(),
                    "Today priority",
                )?,
                priority_tomorrow: parse_ntfy_priority(
                    form.ntfy_priority_tomorrow.as_deref(),
                    "Tomorrow priority",
                )?,
                auth_type,
                username: match auth_type {
                    NtfyAuthType::Basic => {
                        Some(required_field(form.ntfy_username.as_deref(), "Username")?)
                    }
                    _ => None,
                },
                password: match auth_type {
                    NtfyAuthType::Basic => {
                        Some(required_field(form.ntfy_password.as_deref(), "Password")?)
                    }
                    _ => None,
                },
                token: match auth_type {
                    NtfyAuthType::Bearer => {
                        Some(required_field(form.ntfy_token.as_deref(), "Bearer token")?)
                    }
                    _ => None,
                },
            };
            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
        ChannelKind::Pushover => {
            let cfg = PushoverConfig {
                api_token: required_field(form.pushover_api_token.as_deref(), "API Token")?,
                user_key: required_field(form.pushover_user_key.as_deref(), "User Key")?,
            };
            serde_json::to_value(cfg).map_err(|e| format!("Failed to encode config: {}", e))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn channel_form_template(
    user: User,
    channel_type: String,
    kind: ChannelKind,
    existing: Option<&NotificationChannelRecord>,
    enabled: bool,
    form: ChannelConfigForm,
    has_existing: bool,
    error: Option<String>,
    success: Option<String>,
    csrf_token: String,
) -> ChannelFormTemplate {
    let mut template = ChannelFormTemplate {
        user,
        channel_type,
        channel_name: kind.display_name().to_string(),
        enabled,
        has_existing,
        gotify_url: trim_or_empty(form.gotify_url.as_deref()),
        gotify_token: trim_or_empty(form.gotify_token.as_deref()),
        email_provider: trim_or_empty(form.email_provider.as_deref()),
        email_username: trim_or_empty(form.email_username.as_deref()),
        email_password: trim_or_empty(form.email_password.as_deref()),
        email_to: trim_or_empty(form.email_to.as_deref()),
        email_smtp_host: trim_or_empty(form.email_smtp_host.as_deref()),
        email_smtp_port: trim_or_empty(form.email_smtp_port.as_deref()),
        email_smtp_security: trim_or_empty(form.email_smtp_security.as_deref()),
        telegram_bot_token: trim_or_empty(form.telegram_bot_token.as_deref()),
        telegram_chat_id: trim_or_empty(form.telegram_chat_id.as_deref()),
        signal_api_url: trim_or_empty(form.signal_api_url.as_deref()),
        signal_recipient: trim_or_empty(form.signal_recipient.as_deref()),
        whatsapp_api_url: trim_or_empty(form.whatsapp_api_url.as_deref()),
        whatsapp_recipient: trim_or_empty(form.whatsapp_recipient.as_deref()),
        discord_webhook_url: trim_or_empty(form.discord_webhook_url.as_deref()),
        sms_account_sid: trim_or_empty(form.sms_account_sid.as_deref()),
        sms_auth_token: trim_or_empty(form.sms_auth_token.as_deref()),
        sms_from_number: trim_or_empty(form.sms_from_number.as_deref()),
        sms_to_number: trim_or_empty(form.sms_to_number.as_deref()),
        ntfy_server_url: trim_or_empty(form.ntfy_server_url.as_deref()),
        ntfy_topic: trim_or_empty(form.ntfy_topic.as_deref()),
        ntfy_priority_default: trim_or_empty(form.ntfy_priority_default.as_deref()),
        ntfy_priority_today: trim_or_empty(form.ntfy_priority_today.as_deref()),
        ntfy_priority_tomorrow: trim_or_empty(form.ntfy_priority_tomorrow.as_deref()),
        ntfy_auth_type: trim_or_empty(form.ntfy_auth_type.as_deref()),
        ntfy_username: trim_or_empty(form.ntfy_username.as_deref()),
        ntfy_password: trim_or_empty(form.ntfy_password.as_deref()),
        ntfy_token: trim_or_empty(form.ntfy_token.as_deref()),
        pushover_api_token: trim_or_empty(form.pushover_api_token.as_deref()),
        pushover_user_key: trim_or_empty(form.pushover_user_key.as_deref()),
        error,
        success,
        csrf_token,
    };

    if let Some(existing) = existing {
        match kind {
            ChannelKind::Gotify => {
                if let Ok(cfg) = serde_json::from_value::<GotifyConfig>(existing.config.clone()) {
                    if template.gotify_url.is_empty() {
                        template.gotify_url = cfg.url;
                    }
                    if template.gotify_token.is_empty() {
                        template.gotify_token = cfg.token;
                    }
                }
            }
            ChannelKind::Email => {
                if let Ok(cfg) = serde_json::from_value::<EmailConfig>(existing.config.clone()) {
                    if template.email_provider.is_empty() {
                        template.email_provider = email_provider_as_str(cfg.provider).to_string();
                    }
                    if template.email_username.is_empty() {
                        template.email_username = cfg.username;
                    }
                    if template.email_password.is_empty() {
                        template.email_password = cfg.password;
                    }
                    if template.email_to.is_empty() {
                        template.email_to = cfg.to;
                    }
                    if template.email_smtp_host.is_empty() {
                        template.email_smtp_host = cfg.smtp_host.unwrap_or_default();
                    }
                    if template.email_smtp_port.is_empty() {
                        template.email_smtp_port =
                            cfg.smtp_port.map(|p| p.to_string()).unwrap_or_default();
                    }
                    if template.email_smtp_security.is_empty() {
                        template.email_smtp_security = cfg
                            .security
                            .map(smtp_security_as_str)
                            .unwrap_or("starttls")
                            .to_string();
                    }
                }
            }
            ChannelKind::Telegram => {
                if let Ok(cfg) = serde_json::from_value::<TelegramConfig>(existing.config.clone()) {
                    if template.telegram_bot_token.is_empty() {
                        template.telegram_bot_token = cfg.bot_token;
                    }
                    if template.telegram_chat_id.is_empty() {
                        template.telegram_chat_id = cfg.chat_id;
                    }
                }
            }
            ChannelKind::Signal => {
                if let Ok(cfg) = serde_json::from_value::<SignalConfig>(existing.config.clone()) {
                    if template.signal_api_url.is_empty() {
                        template.signal_api_url = cfg.api_url;
                    }
                    if template.signal_recipient.is_empty() {
                        template.signal_recipient = cfg.recipient;
                    }
                }
            }
            ChannelKind::Whatsapp => {
                if let Ok(cfg) = serde_json::from_value::<WhatsappConfig>(existing.config.clone()) {
                    if template.whatsapp_api_url.is_empty() {
                        template.whatsapp_api_url = cfg.api_url;
                    }
                    if template.whatsapp_recipient.is_empty() {
                        template.whatsapp_recipient = cfg.recipient;
                    }
                }
            }
            ChannelKind::Discord => {
                if let Ok(cfg) = serde_json::from_value::<DiscordConfig>(existing.config.clone())
                    && template.discord_webhook_url.is_empty()
                {
                    template.discord_webhook_url = cfg.webhook_url;
                }
            }
            ChannelKind::Sms => {
                if let Ok(cfg) = serde_json::from_value::<SmsConfig>(existing.config.clone()) {
                    if template.sms_account_sid.is_empty() {
                        template.sms_account_sid = cfg.account_sid;
                    }
                    if template.sms_auth_token.is_empty() {
                        template.sms_auth_token = cfg.auth_token;
                    }
                    if template.sms_from_number.is_empty() {
                        template.sms_from_number = cfg.from_number;
                    }
                    if template.sms_to_number.is_empty() {
                        template.sms_to_number = cfg.to_number;
                    }
                }
            }
            ChannelKind::Ntfy => {
                if let Ok(cfg) = serde_json::from_value::<NtfyConfig>(existing.config.clone()) {
                    if template.ntfy_server_url.is_empty() {
                        template.ntfy_server_url = cfg.server_url;
                    }
                    if template.ntfy_topic.is_empty() {
                        template.ntfy_topic = cfg.topic;
                    }
                    if template.ntfy_auth_type.is_empty() {
                        template.ntfy_auth_type = ntfy_auth_type_as_str(cfg.auth_type).to_string();
                    }
                    if template.ntfy_priority_default.is_empty() {
                        template.ntfy_priority_default = cfg.priority_default.to_string();
                    }
                    if template.ntfy_priority_today.is_empty() {
                        template.ntfy_priority_today = cfg
                            .priority_today
                            .map(|p| p.to_string())
                            .unwrap_or_default();
                    }
                    if template.ntfy_priority_tomorrow.is_empty() {
                        template.ntfy_priority_tomorrow = cfg
                            .priority_tomorrow
                            .map(|p| p.to_string())
                            .unwrap_or_default();
                    }
                    if template.ntfy_username.is_empty() {
                        template.ntfy_username = cfg.username.unwrap_or_default();
                    }
                    if template.ntfy_password.is_empty() {
                        template.ntfy_password = cfg.password.unwrap_or_default();
                    }
                    if template.ntfy_token.is_empty() {
                        template.ntfy_token = cfg.token.unwrap_or_default();
                    }
                }
            }
            ChannelKind::Pushover => {
                if let Ok(cfg) = serde_json::from_value::<PushoverConfig>(existing.config.clone()) {
                    if template.pushover_api_token.is_empty() {
                        template.pushover_api_token = cfg.api_token;
                    }
                    if template.pushover_user_key.is_empty() {
                        template.pushover_user_key = cfg.user_key;
                    }
                }
            }
        }
    }
    template
}

pub async fn save_channel(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Path(channel_type): Path<String>,
    Form(form): Form<ChannelConfigForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let kind = match ChannelKind::from_str(&channel_type) {
        Some(k) => k,
        None => return Redirect::to("/notifications").into_response(),
    };

    let existing = state
        .notification_service
        .list_channels(&user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.channel_type == channel_type);

    let config: serde_json::Value = match build_config(kind.clone(), &form) {
        Ok(v) => v,
        Err(e) => {
            let enabled = form.enabled.is_some();
            let template = channel_form_template(
                user,
                channel_type,
                kind,
                existing.as_ref(),
                enabled,
                form,
                existing.is_some(),
                Some(e),
                None,
                csrf_token,
            );
            return Html(template.to_string()).into_response();
        }
    };

    let enabled = form.enabled.is_some();

    match state
        .notification_service
        .upsert_channel(&user.id, &channel_type, enabled, config.clone())
        .await
    {
        Ok(_) => {
            let template = channel_form_template(
                user,
                channel_type,
                kind,
                None,
                enabled,
                form,
                true,
                None,
                Some("Channel saved successfully".to_string()),
                csrf_token,
            );
            Html(template.to_string()).into_response()
        }
        Err(e) => {
            let template = channel_form_template(
                user,
                channel_type,
                kind,
                None,
                enabled,
                form,
                true,
                Some(e.to_string()),
                None,
                csrf_token,
            );
            Html(template.to_string()).into_response()
        }
    }
}

pub async fn test_channel(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Path(channel_type): Path<String>,
    Form(form): Form<TestChannelForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let kind = match ChannelKind::from_str(&channel_type) {
        Some(k) => k,
        None => return Redirect::to("/notifications").into_response(),
    };

    let existing = state
        .notification_service
        .list_channels(&user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.channel_type == channel_type);

    let result = state
        .notification_service
        .test_channel(&user.id, &channel_type)
        .await;

    if form.source.as_deref() == Some("list") {
        let display_name = kind.display_name();
        return match result {
            Ok(()) => {
                let msg = url::form_urlencoded::byte_serialize(
                    format!("Test notification sent via {}!", display_name).as_bytes(),
                )
                .collect::<String>();
                Redirect::to(&format!("/notifications?test_ok={}", msg)).into_response()
            }
            Err(e) => {
                let msg = url::form_urlencoded::byte_serialize(
                    format!("Test failed for {}: {}", display_name, e).as_bytes(),
                )
                .collect::<String>();
                Redirect::to(&format!("/notifications?test_err={}", msg)).into_response()
            }
        };
    }

    let (error, success) = match result {
        Ok(()) => (
            None,
            Some("Test notification sent successfully!".to_string()),
        ),
        Err(e) => (Some(format!("Test failed: {}", e)), None),
    };

    let template = channel_form_template(
        user,
        channel_type,
        kind,
        existing.as_ref(),
        existing.as_ref().map(|r| r.enabled).unwrap_or(true),
        ChannelConfigForm::default(),
        existing.is_some(),
        error,
        success,
        csrf_token,
    );
    Html(template.to_string()).into_response()
}

pub async fn delete_channel(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(channel_type): Path<String>,
) -> impl IntoResponse {
    let _ = state
        .notification_service
        .delete_channel(&user.id, &channel_type)
        .await;
    Redirect::to("/notifications")
}
