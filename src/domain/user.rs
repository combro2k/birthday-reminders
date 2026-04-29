use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    #[cfg(test)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl From<Uuid> for UserId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    User,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::User => "user",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "admin" => Role::Admin,
            _ => Role::User,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    Local,
    Oidc,
    Both,
}

impl AuthMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthMethod::Local => "local",
            AuthMethod::Oidc => "oidc",
            AuthMethod::Both => "both",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "oidc" => AuthMethod::Oidc,
            "both" => AuthMethod::Both,
            _ => AuthMethod::Local,
        }
    }

    pub fn can_login_with_password(&self) -> bool {
        matches!(self, AuthMethod::Local | AuthMethod::Both)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub role: Role,
    pub auth_method: AuthMethod,
    pub oidc_subject: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}
