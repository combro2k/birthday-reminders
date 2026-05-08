use askama::Template;

use crate::infrastructure::web::templates::AppVersion;
use crate::users::domain::user::User;

#[derive(Template)]
#[template(path = "notifications/channels.html")]
pub struct ChannelsTemplate {
    pub user: User,
    pub groups: Vec<ChannelGroupView>,
    pub csrf_token: String,
    pub test_success: Option<String>,
    pub test_error: Option<String>,
}

#[derive(Template)]
#[template(path = "notifications/channel_form.html")]
pub struct ChannelFormTemplate {
    pub user: User,
    pub channel_type: String,
    pub channel_name: String,
    pub enabled: bool,
    pub has_existing: bool,
    pub gotify_url: String,
    pub gotify_token: String,
    pub email_provider: String,
    pub email_username: String,
    pub email_password: String,
    pub email_to: String,
    pub email_smtp_host: String,
    pub email_smtp_port: String,
    pub email_smtp_security: String,
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
    pub signal_api_url: String,
    pub signal_recipient: String,
    pub whatsapp_phone_number_id: String,
    pub whatsapp_access_token: String,
    pub whatsapp_recipient_phone: String,
    pub discord_webhook_url: String,
    pub sms_account_sid: String,
    pub sms_auth_token: String,
    pub sms_from_number: String,
    pub sms_to_number: String,
    pub ntfy_server_url: String,
    pub ntfy_topic: String,
    pub ntfy_priority_default: String,
    pub ntfy_priority_today: String,
    pub ntfy_priority_tomorrow: String,
    pub ntfy_auth_type: String,
    pub ntfy_username: String,
    pub ntfy_password: String,
    pub ntfy_token: String,
    pub pushover_api_token: String,
    pub pushover_user_key: String,
    pub error: Option<String>,
    pub success: Option<String>,
    pub csrf_token: String,
}

impl AppVersion for ChannelsTemplate {}
impl AppVersion for ChannelFormTemplate {}

// ---- View Models ----

#[derive(Debug, Clone)]
pub struct ChannelGroupView {
    pub label: String,
    pub channels: Vec<ChannelKindView>,
}

#[derive(Debug, Clone)]
pub struct ChannelKindView {
    pub kind: String,
    pub display_name: String,
    pub configured: bool,
    /// `Some(true/false)` = configured + enabled state; `None` = not configured
    pub enabled: Option<bool>,
    /// False for channels that are stubbed but not yet fully implemented
    pub implemented: bool,
}
