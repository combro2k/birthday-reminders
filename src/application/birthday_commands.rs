use std::sync::Arc;

use chrono::NaiveDate;

use crate::domain::birthday::Birthday;
use crate::domain::repository::{BirthdayRepository, NewBirthday, UpdateBirthday};
use crate::domain::user::UserId;

pub struct BirthdayCommandService {
    repo: Arc<dyn BirthdayRepository>,
}

impl BirthdayCommandService {
    pub fn new(repo: Arc<dyn BirthdayRepository>) -> Self {
        Self { repo }
    }

    pub async fn add(
        &self,
        user_id: &UserId,
        name: &str,
        birth_date: NaiveDate,
        notes: Option<String>,
    ) -> anyhow::Result<Birthday> {
        validate_birthday_input(name, birth_date, notes.as_deref())?;

        let new = NewBirthday {
            user_id: user_id.clone(),
            name: name.to_string(),
            birth_date,
            notes,
        };
        let birthday = self.repo.create(new).await.map_err(|e| {
            anyhow::anyhow!("Failed to create birthday: {}", e)
        })?;
        Ok(birthday)
    }

    pub async fn update(
        &self,
        id: uuid::Uuid,
        user_id: &UserId,
        name: Option<String>,
        birth_date: Option<NaiveDate>,
        notes: Option<Option<String>>,
    ) -> anyhow::Result<Birthday> {
        // Validate provided fields
        let effective_name = name.as_deref().unwrap_or("placeholder");
        let effective_date = birth_date.unwrap_or(chrono::Local::now().date_naive());
        let effective_notes = notes.as_ref().and_then(|n| n.as_deref());
        validate_birthday_input(effective_name, effective_date, effective_notes)?;

        // Verify ownership
        let existing = self
            .repo
            .find_by_id(&id.into())
            .await
            .map_err(|e| anyhow::anyhow!("Birthday not found: {}", e))?;

        if existing.user_id != *user_id {
            anyhow::bail!("Not authorized to modify this birthday");
        }

        let update = UpdateBirthday {
            name,
            birth_date,
            notes,
        };
        let birthday = self.repo.update(&id.into(), update).await.map_err(|e| {
            anyhow::anyhow!("Failed to update birthday: {}", e)
        })?;
        Ok(birthday)
    }

    pub async fn delete(&self, id: uuid::Uuid, user_id: &UserId) -> anyhow::Result<()> {
        // Verify ownership
        let existing = self
            .repo
            .find_by_id(&id.into())
            .await
            .map_err(|e| anyhow::anyhow!("Birthday not found: {}", e))?;

        if existing.user_id != *user_id {
            anyhow::bail!("Not authorized to delete this birthday");
        }

        self.repo.delete(&id.into()).await.map_err(|e| {
            anyhow::anyhow!("Failed to delete birthday: {}", e)
        })?;
        Ok(())
    }
}

fn validate_birthday_input(name: &str, birth_date: NaiveDate, notes: Option<&str>) -> anyhow::Result<()> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("Name cannot be empty");
    }
    if name.len() > 200 {
        anyhow::bail!("Name cannot exceed 200 characters");
    }

    let today = chrono::Local::now().date_naive();
    if birth_date > today {
        anyhow::bail!("Birth date cannot be in the future");
    }

    if let Some(n) = notes {
        if n.len() > 2000 {
            anyhow::bail!("Notes cannot exceed 2000 characters");
        }
    }

    Ok(())
}
