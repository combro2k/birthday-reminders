use async_trait::async_trait;
use chrono::NaiveDate;

use crate::infrastructure::error::RepositoryError;
use crate::users::domain::user::UserId;

use super::birthday::{Birthday, BirthdayId};

#[derive(Debug, Clone)]
pub struct NewBirthday {
    pub user_id: UserId,
    pub name: String,
    pub birth_date: NaiveDate,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateBirthday {
    pub name: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub email: Option<Option<String>>,
    pub phone_number: Option<Option<String>>,
    pub address: Option<Option<String>>,
    pub postal_code: Option<Option<String>>,
    pub city: Option<Option<String>>,
    pub country: Option<Option<String>>,
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
