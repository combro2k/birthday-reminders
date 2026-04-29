use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::domain::birthday::{Birthday, BirthdayId};
use crate::domain::repository::{BirthdayRepository, NewBirthday, RepositoryError, UpdateBirthday};
use crate::domain::user::UserId;

pub struct SqliteBirthdayRepo {
    pool: SqlitePool,
}

impl SqliteBirthdayRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BirthdayRow {
    id: String,
    user_id: String,
    name: String,
    birth_date: String,
    notes: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<BirthdayRow> for Birthday {
    type Error = RepositoryError;

    fn try_from(row: BirthdayRow) -> Result<Self, Self::Error> {
        Ok(Birthday {
            id: BirthdayId(
                Uuid::parse_str(&row.id).map_err(|e| RepositoryError::Database(e.to_string()))?,
            ),
            user_id: UserId(
                Uuid::parse_str(&row.user_id)
                    .map_err(|e| RepositoryError::Database(e.to_string()))?,
            ),
            name: row.name,
            birth_date: chrono::NaiveDate::parse_from_str(&row.birth_date, "%Y-%m-%d")
                .map_err(|e| RepositoryError::Database(e.to_string()))?,
            notes: row.notes,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }
}

#[async_trait]
impl BirthdayRepository for SqliteBirthdayRepo {
    async fn create(&self, new: NewBirthday) -> Result<Birthday, RepositoryError> {
        let id = Uuid::new_v4().to_string();
        let user_id = new.user_id.0.to_string();
        let birth_date = new.birth_date.format("%Y-%m-%d").to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO birthdays (id, user_id, name, birth_date, notes, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&new.name)
        .bind(&birth_date)
        .bind(&new.notes)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(Birthday {
            id: BirthdayId(Uuid::parse_str(&id).unwrap()),
            user_id: new.user_id,
            name: new.name,
            birth_date: new.birth_date,
            notes: new.notes,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    async fn find_by_id(&self, id: &BirthdayId) -> Result<Birthday, RepositoryError> {
        let row = sqlx::query_as::<_, BirthdayRow>(
            "SELECT id, user_id, name, birth_date, notes, created_at, updated_at
             FROM birthdays WHERE id = ?",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;
        row.try_into()
    }

    async fn find_all_for_user(&self, user_id: &UserId) -> Result<Vec<Birthday>, RepositoryError> {
        let rows = sqlx::query_as::<_, BirthdayRow>(
            "SELECT id, user_id, name, birth_date, notes, created_at, updated_at
             FROM birthdays WHERE user_id = ? ORDER BY birth_date",
        )
        .bind(user_id.0.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn update(
        &self,
        id: &BirthdayId,
        update: UpdateBirthday,
    ) -> Result<Birthday, RepositoryError> {
        let current = self.find_by_id(id).await?;

        let name = update.name.unwrap_or(current.name);
        let birth_date = update.birth_date.unwrap_or(current.birth_date);
        let notes = match update.notes {
            Some(n) => n,
            None => current.notes,
        };
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE birthdays SET name = ?, birth_date = ?, notes = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&name)
        .bind(birth_date.format("%Y-%m-%d").to_string())
        .bind(&notes)
        .bind(&now)
        .bind(id.0.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        self.find_by_id(id).await
    }

    async fn delete(&self, id: &BirthdayId) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM birthdays WHERE id = ?")
            .bind(id.0.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn find_upcoming(
        &self,
        user_id: &UserId,
        within_days: u32,
    ) -> Result<Vec<Birthday>, RepositoryError> {
        // Fetch all birthdays for user and filter in Rust (SQLite date functions are limited)
        let all = self.find_all_for_user(user_id).await?;
        let today = chrono::Local::now().date_naive();

        Ok(all
            .into_iter()
            .filter(|b| {
                let this_year = today.year();
                let next_birthday = b.birth_date.with_year(this_year).unwrap_or(b.birth_date);
                let next_birthday = if next_birthday < today {
                    b.birth_date
                        .with_year(this_year + 1)
                        .unwrap_or(next_birthday)
                } else {
                    next_birthday
                };
                let days_until = (next_birthday - today).num_days();
                days_until >= 0 && days_until <= within_days as i64
            })
            .collect())
    }

    async fn has_been_reminded(
        &self,
        birthday_id: &BirthdayId,
        channel_type: &str,
        days_before: u32,
        year: i32,
    ) -> Result<bool, RepositoryError> {
        let count = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM reminder_log
             WHERE birthday_id = ? AND channel_type = ? AND days_before = ? AND year = ?",
        )
        .bind(birthday_id.0.to_string())
        .bind(channel_type)
        .bind(days_before as i32)
        .bind(year)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    async fn log_reminder(
        &self,
        birthday_id: &BirthdayId,
        channel_type: &str,
        days_before: u32,
        year: i32,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT OR IGNORE INTO reminder_log (birthday_id, channel_type, days_before, year)
             VALUES (?, ?, ?, ?)",
        )
        .bind(birthday_id.0.to_string())
        .bind(channel_type)
        .bind(days_before as i32)
        .bind(year)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }

    async fn cleanup_old_reminders(&self, older_than_days: u32) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM reminder_log WHERE reminded_at < datetime('now', '-' || ? || ' days')",
        )
        .bind(older_than_days as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(result.rows_affected())
    }
}

use chrono::Datelike;
