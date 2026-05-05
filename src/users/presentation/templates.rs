use askama::Template;

use crate::infrastructure::web::templates::AppVersion;
use crate::users::application::commands::ApiTokenInfo;
use crate::users::domain::user::User;

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

#[derive(Template)]
#[template(path = "admin/users.html")]
pub struct AdminUsersTemplate {
    pub user: User,
    pub users: Vec<UserView>,
    pub error: Option<String>,
    pub success: Option<String>,
    pub csrf_token: String,
}

impl AppVersion for ProfileTemplate {}
impl AppVersion for ApiTokensTemplate {}
impl AppVersion for AdminUsersTemplate {}

// ---- View Models ----

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
