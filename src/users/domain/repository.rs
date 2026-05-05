use async_trait::async_trait;

use crate::infrastructure::error::RepositoryError;

use super::user::{AuthMethod, Role, User, UserId};

#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub role: Role,
    pub auth_method: AuthMethod,
    pub oidc_subject: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<Option<String>>,
    pub role: Option<Role>,
    pub auth_method: Option<AuthMethod>,
    pub oidc_subject: Option<Option<String>>,
    pub date_format: Option<String>,
    pub theme: Option<String>,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, new: NewUser) -> Result<User, RepositoryError>;
    async fn find_by_id(&self, id: &UserId) -> Result<User, RepositoryError>;
    async fn find_by_username(&self, username: &str) -> Result<User, RepositoryError>;
    async fn find_by_oidc_subject(&self, subject: &str) -> Result<User, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<User>, RepositoryError>;
    async fn update(&self, id: &UserId, update: UpdateUser) -> Result<User, RepositoryError>;
    async fn delete(&self, id: &UserId) -> Result<(), RepositoryError>;

    /// Get user's reminder day preferences (None = use global default)
    async fn get_reminder_days(
        &self,
        user_id: &UserId,
    ) -> Result<Option<Vec<i32>>, RepositoryError>;
    async fn set_reminder_days(
        &self,
        user_id: &UserId,
        days: Vec<i32>,
    ) -> Result<(), RepositoryError>;
}
