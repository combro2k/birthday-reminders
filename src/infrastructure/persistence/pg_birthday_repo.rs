use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::birthday::{Birthday, BirthdayId};
use crate::domain::repository::{BirthdayRepository, NewBirthday, RepositoryError, UpdateBirthday};
use crate::domain::user::UserId;

pub struct PgBirthdayRepo {
    pool: PgPool,
}

impl PgBirthdayRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BirthdayRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    birth_date: chrono::NaiveDate,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<BirthdayRow> for Birthday {
    fn from(row: BirthdayRow) -> Self {
        Birthday {
            id: BirthdayId(row.id),
            user_id: UserId(row.user_id),
            name: row.name,
            birth_date: row.birth_date,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl BirthdayRepository for PgBirthdayRepo {
    async fn create(&self, new: NewBirthday) -> Result<Birthday, RepositoryError> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, BirthdayRow>(
            "INSERT INTO birthdays (id, user_id, name, birth_date, notes)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, user_id, name, birth_date, notes, created_at, updated_at",
        )
        .bind(id)
        .bind(new.user_id.0)
        .bind(&new.name)
        .bind(new.birth_date)
        .bind(&new.notes)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: &BirthdayId) -> Result<Birthday, RepositoryError> {
        let row = sqlx::query_as::<_, BirthdayRow>(
            "SELECT id, user_id, name, birth_date, notes, created_at, updated_at
             FROM birthdays WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;
        Ok(row.into())
    }

    async fn find_all_for_user(&self, user_id: &UserId) -> Result<Vec<Birthday>, RepositoryError> {
        let rows = sqlx::query_as::<_, BirthdayRow>(
            "SELECT id, user_id, name, birth_date, notes, created_at, updated_at
             FROM birthdays WHERE user_id = $1 ORDER BY birth_date",
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
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

        let row = sqlx::query_as::<_, BirthdayRow>(
            "UPDATE birthdays SET name = $1, birth_date = $2, notes = $3, updated_at = NOW()
             WHERE id = $4
             RETURNING id, user_id, name, birth_date, notes, created_at, updated_at",
        )
        .bind(&name)
        .bind(birth_date)
        .bind(&notes)
        .bind(id.0)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(row.into())
    }

    async fn delete(&self, id: &BirthdayId) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM birthdays WHERE id = $1")
            .bind(id.0)
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
        // Use SQL to compute next birthday occurrence and filter within range
        let rows = sqlx::query_as::<_, BirthdayRow>(
            "SELECT id, user_id, name, birth_date, notes, created_at, updated_at
             FROM birthdays
             WHERE user_id = $1
               AND (
                 -- This year's birthday hasn't passed yet
                 (MAKE_DATE(EXTRACT(YEAR FROM CURRENT_DATE)::int,
                            EXTRACT(MONTH FROM birth_date)::int,
                            EXTRACT(DAY FROM birth_date)::int) - CURRENT_DATE
                  BETWEEN 0 AND $2)
                 OR
                 -- Next year's birthday is within range
                 (MAKE_DATE(EXTRACT(YEAR FROM CURRENT_DATE)::int + 1,
                            EXTRACT(MONTH FROM birth_date)::int,
                            EXTRACT(DAY FROM birth_date)::int) - CURRENT_DATE
                  BETWEEN 0 AND $2)
               )
             ORDER BY
               CASE
                 WHEN MAKE_DATE(EXTRACT(YEAR FROM CURRENT_DATE)::int,
                                EXTRACT(MONTH FROM birth_date)::int,
                                EXTRACT(DAY FROM birth_date)::int) >= CURRENT_DATE
                 THEN MAKE_DATE(EXTRACT(YEAR FROM CURRENT_DATE)::int,
                                EXTRACT(MONTH FROM birth_date)::int,
                                EXTRACT(DAY FROM birth_date)::int) - CURRENT_DATE
                 ELSE MAKE_DATE(EXTRACT(YEAR FROM CURRENT_DATE)::int + 1,
                                EXTRACT(MONTH FROM birth_date)::int,
                                EXTRACT(DAY FROM birth_date)::int) - CURRENT_DATE
               END",
        )
        .bind(user_id.0)
        .bind(within_days as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn has_been_reminded(
        &self,
        birthday_id: &BirthdayId,
        channel_type: &str,
        days_before: u32,
        year: i32,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1 FROM reminder_log
                WHERE birthday_id = $1 AND channel_type = $2 AND days_before = $3 AND year = $4
            )",
        )
        .bind(birthday_id.0)
        .bind(channel_type)
        .bind(days_before as i32)
        .bind(year)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(row)
    }

    async fn log_reminder(
        &self,
        birthday_id: &BirthdayId,
        channel_type: &str,
        days_before: u32,
        year: i32,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO reminder_log (birthday_id, channel_type, days_before, year)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (birthday_id, channel_type, days_before, year) DO NOTHING",
        )
        .bind(birthday_id.0)
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
            "DELETE FROM reminder_log WHERE reminded_at < NOW() - make_interval(days => $1)",
        )
        .bind(older_than_days as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(result.rows_affected())
    }
}
