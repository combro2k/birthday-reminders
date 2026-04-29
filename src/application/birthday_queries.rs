use std::sync::Arc;

use crate::domain::birthday::Birthday;
use crate::domain::repository::BirthdayRepository;
use crate::domain::user::UserId;

pub struct BirthdayQueryService {
    repo: Arc<dyn BirthdayRepository>,
}

impl BirthdayQueryService {
    pub fn new(repo: Arc<dyn BirthdayRepository>) -> Self {
        Self { repo }
    }

    pub async fn list_all(&self, user_id: &UserId) -> anyhow::Result<Vec<Birthday>> {
        let birthdays = self
            .repo
            .find_all_for_user(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list birthdays: {}", e))?;
        Ok(birthdays)
    }

    pub async fn get_upcoming(
        &self,
        user_id: &UserId,
        within_days: u32,
    ) -> anyhow::Result<Vec<Birthday>> {
        let birthdays = self
            .repo
            .find_upcoming(user_id, within_days)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get upcoming birthdays: {}", e))?;
        Ok(birthdays)
    }

    pub async fn get_by_id(&self, id: uuid::Uuid, user_id: &UserId) -> anyhow::Result<Birthday> {
        let birthday = self
            .repo
            .find_by_id(&id.into())
            .await
            .map_err(|e| anyhow::anyhow!("Birthday not found: {}", e))?;
        if birthday.user_id != *user_id {
            anyhow::bail!("Not authorized to view this birthday");
        }
        Ok(birthday)
    }
}
