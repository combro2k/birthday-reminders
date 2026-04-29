use async_trait::async_trait;
use chrono::NaiveDate;
use uuid::Uuid;

use super::birthday::{Birthday, BirthdayId};
use super::user::UserId;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Not found")]
    NotFound,

    #[error("Already exists")]
    AlreadyExists,

    #[error("Database error: {0}")]
    Database(String),
}

#[derive(Debug, Clone)]
pub struct NewBirthday {
    pub user_id: UserId,
    pub name: String,
    pub birth_date: NaiveDate,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateBirthday {
    pub name: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub notes: Option<Option<String>>,
}

#[async_trait]
pub trait BirthdayRepository: Send + Sync {
    async fn create(&self, new: NewBirthday) -> Result<Birthday, RepositoryError>;
    async fn find_by_id(&self, id: &BirthdayId) -> Result<Birthday, RepositoryError>;
    async fn find_all_for_user(&self, user_id: &UserId) -> Result<Vec<Birthday>, RepositoryError>;
    async fn update(
        &self,
        id: &BirthdayId,
        update: UpdateBirthday,
    ) -> Result<Birthday, RepositoryError>;
    async fn delete(&self, id: &BirthdayId) -> Result<(), RepositoryError>;

    /// Find birthdays where the next occurrence is within `days` days from today
    async fn find_upcoming(
        &self,
        user_id: &UserId,
        within_days: u32,
    ) -> Result<Vec<Birthday>, RepositoryError>;

    /// Check if a reminder has already been logged for this birthday/channel/days_before/year
    async fn has_been_reminded(
        &self,
        birthday_id: &BirthdayId,
        channel_type: &str,
        days_before: u32,
        year: i32,
    ) -> Result<bool, RepositoryError>;

    /// Log that a reminder was sent
    async fn log_reminder(
        &self,
        birthday_id: &BirthdayId,
        channel_type: &str,
        days_before: u32,
        year: i32,
    ) -> Result<(), RepositoryError>;

    /// Delete reminder log entries older than the given number of days
    async fn cleanup_old_reminders(&self, older_than_days: u32) -> Result<u64, RepositoryError>;
}

/// Notification channel persistence
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NotificationChannelRecord {
    pub id: Uuid,
    pub user_id: UserId,
    pub channel_type: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait NotificationChannelRepository: Send + Sync {
    async fn find_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<NotificationChannelRecord>, RepositoryError>;

    async fn find_by_type(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<Option<NotificationChannelRecord>, RepositoryError>;

    async fn upsert(
        &self,
        user_id: &UserId,
        channel_type: &str,
        enabled: bool,
        config: serde_json::Value,
    ) -> Result<NotificationChannelRecord, RepositoryError>;

    async fn delete(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<(), RepositoryError>;

    async fn find_enabled_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<NotificationChannelRecord>, RepositoryError>;
}
