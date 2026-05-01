use askama::Template;
use chrono::NaiveDate;

use crate::application::user_commands::ApiTokenInfo;
use crate::domain::birthday::Birthday;
use crate::domain::notification::ChannelKind;
use crate::domain::repository::NotificationChannelRecord;
use crate::domain::user::User;

// ---- Auth Templates ----

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
    pub oidc_enabled: bool,
    pub oidc_provider_name: String,
    pub registration_enabled: bool,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub error: Option<String>,
    pub csrf_token: String,
}

// ---- Birthday Templates ----

#[derive(Template)]
#[template(path = "birthdays/index.html")]
pub struct DashboardTemplate {
    pub user: User,
    pub upcoming: Vec<BirthdayView>,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "birthdays/list.html")]
pub struct BirthdayListTemplate {
    pub user: User,
    pub birthdays: Vec<BirthdayView>,
    pub current_sort: String,
    pub is_desc: bool,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "birthdays/form.html")]
pub struct BirthdayFormTemplate {
    pub user: User,
    pub birthday: Option<BirthdayView>,
    pub edit_name: String,
    pub edit_date: String,
    pub edit_notes: String,
    pub error: Option<String>,
    pub csrf_token: String,
}

// ---- Notification Templates ----

#[derive(Template)]
#[template(path = "notifications/channels.html")]
pub struct ChannelsTemplate {
    pub user: User,
    pub channels: Vec<ChannelView>,
    pub available: Vec<ChannelKindView>,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "notifications/channel_form.html")]
pub struct ChannelFormTemplate {
    pub user: User,
    pub channel_type: String,
    pub channel_name: String,
    pub config_json: String,
    pub enabled: bool,
    pub has_existing: bool,
    pub error: Option<String>,
    pub success: Option<String>,
    pub csrf_token: String,
}

// ---- Settings Templates ----

#[derive(Template)]
#[template(path = "settings/profile.html")]
pub struct ProfileTemplate {
    pub user: User,
    pub error: Option<String>,
    pub success: Option<String>,
    pub csrf_token: String,
    pub reminder_days: Vec<i32>,
    pub reminder_days_csv: String,
}

#[derive(Template)]
#[template(path = "settings/api_tokens.html")]
pub struct ApiTokensTemplate {
    pub user: User,
    pub tokens: Vec<ApiTokenView>,
    pub new_token: Option<String>,
    pub error: Option<String>,
    pub csrf_token: String,
}

// ---- Admin Templates ----

#[derive(Template)]
#[template(path = "admin/users.html")]
pub struct AdminUsersTemplate {
    pub user: User,
    pub users: Vec<UserView>,
    pub error: Option<String>,
    pub success: Option<String>,
    pub csrf_token: String,
}

// ---- View Models ----

#[derive(Debug, Clone)]
pub struct BirthdayView {
    pub id: uuid::Uuid,
    pub name: String,
    pub birth_date: NaiveDate,
    pub birth_date_str: String,
    pub notes: Option<String>,
    pub age: u32,
    pub turning_age: u32,
    pub days_until: i64,
}

impl BirthdayView {
    pub fn from_birthday(b: Birthday, date_format: &str) -> Self {
        let today = chrono::Local::now().date_naive();
        let format = if date_format.is_empty() {
            "%Y-%m-%d"
        } else {
            date_format
        };

        Self {
            id: b.id.0,
            name: b.name.clone(),
            birth_date: b.birth_date,
            birth_date_str: b.birth_date.format(format).to_string(),
            notes: b.notes.clone(),
            age: b.age_on(today),
            turning_age: b.turning_age_on(today),
            days_until: b.days_until_next_from(today),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelView {
    pub channel_type: String,
    pub display_name: String,
    pub enabled: bool,
}

impl From<NotificationChannelRecord> for ChannelView {
    fn from(r: NotificationChannelRecord) -> Self {
        let display_name = ChannelKind::from_str(&r.channel_type)
            .map(|k| k.display_name().to_string())
            .unwrap_or_else(|| r.channel_type.clone());
        Self {
            channel_type: r.channel_type,
            display_name,
            enabled: r.enabled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelKindView {
    pub kind: String,
    pub display_name: String,
    pub configured: bool,
}

#[derive(Debug, Clone)]
pub struct ApiTokenView {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: String,
    pub last_used_at: String,
}

impl ApiTokenView {
    pub fn from_token_info(t: ApiTokenInfo, date_format: &str) -> Self {
        let format = if date_format.is_empty() {
            "%Y-%m-%d %H:%M"
        } else {
            // Append time format to user's date format
            date_format
        };
        
        Self {
            id: t.id,
            name: t.name,
            created_at: t.created_at.format(format).to_string(),
            last_used_at: t
                .last_used_at
                .map(|d| d.format(format).to_string())
                .unwrap_or_else(|| "Never".to_string()),
        }
    }
}

impl From<ApiTokenInfo> for ApiTokenView {
    fn from(t: ApiTokenInfo) -> Self {
        Self::from_token_info(t, "%Y-%m-%d %H:%M")
    }
}

#[derive(Debug, Clone)]
pub struct UserView {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub auth_method: String,
    pub created_at: String,
}

impl UserView {
    pub fn from_user(u: User, date_format: &str) -> Self {
        let format = if date_format.is_empty() {
            "%Y-%m-%d"
        } else {
            date_format
        };

        Self {
            id: u.id.0,
            username: u.username,
            email: u.email,
            role: u.role.as_str().to_string(),
            auth_method: u.auth_method.as_str().to_string(),
            created_at: u.created_at.format(format).to_string(),
        }
    }
}
