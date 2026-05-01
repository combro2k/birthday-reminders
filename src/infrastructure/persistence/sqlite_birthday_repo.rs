use async_trait::async_trait;
use chrono::Datelike;
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
    id: Uuid,
    user_id: Uuid,
    name: String,
    birth_date: chrono::NaiveDate,
    phone_number: Option<String>,
    address: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    country: Option<String>,
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
            phone_number: row.phone_number,
            address: row.address,
            postal_code: row.postal_code,
            city: row.city,
            country: row.country,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl BirthdayRepository for SqliteBirthdayRepo {
    async fn create(&self, new: NewBirthday) -> Result<Birthday, RepositoryError> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        sqlx::query(
              "INSERT INTO birthdays (id, user_id, name, birth_date, phone_number, address, postal_code, city, country, notes, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(new.user_id.0)
        .bind(&new.name)
        .bind(new.birth_date)
           .bind(&new.phone_number)
           .bind(&new.address)
           .bind(&new.postal_code)
           .bind(&new.city)
           .bind(&new.country)
        .bind(&new.notes)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(Birthday {
            id: BirthdayId(id),
            user_id: new.user_id,
            name: new.name,
            birth_date: new.birth_date,
            phone_number: new.phone_number,
            address: new.address,
            postal_code: new.postal_code,
            city: new.city,
            country: new.country,
            notes: new.notes,
            created_at: now,
            updated_at: now,
        })
    }

    async fn find_by_id(&self, id: &BirthdayId) -> Result<Birthday, RepositoryError> {
        let row = sqlx::query_as::<_, BirthdayRow>(
            "SELECT id, user_id, name, birth_date, phone_number, address, postal_code, city, country, notes, created_at, updated_at
             FROM birthdays WHERE id = ?",
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
            "SELECT id, user_id, name, birth_date, phone_number, address, postal_code, city, country, notes, created_at, updated_at 
             FROM birthdays 
             WHERE user_id = ? 
             ORDER BY strftime('%m', birth_date), strftime('%d', birth_date)",
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
        let phone_number = match update.phone_number {
            Some(n) => n,
            None => current.phone_number,
        };
        let address = match update.address {
            Some(a) => a,
            None => current.address,
        };
        let postal_code = match update.postal_code {
            Some(p) => p,
            None => current.postal_code,
        };
        let city = match update.city {
            Some(c) => c,
            None => current.city,
        };
        let country = match update.country {
            Some(c) => c,
            None => current.country,
        };
        let notes = match update.notes {
            Some(n) => n,
            None => current.notes,
        };
        let now = chrono::Utc::now();

        sqlx::query(
            "UPDATE birthdays SET name = ?, birth_date = ?, phone_number = ?, address = ?, postal_code = ?, city = ?, country = ?, notes = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&name)
        .bind(birth_date)
        .bind(&phone_number)
        .bind(&address)
        .bind(&postal_code)
        .bind(&city)
        .bind(&country)
        .bind(&notes)
        .bind(&now)
        .bind(id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(Birthday {
            id: current.id,
            user_id: current.user_id,
            name,
            birth_date,
            phone_number,
            address,
            postal_code,
            city,
            country,
            notes,
            created_at: current.created_at,
            updated_at: now,
        })
    }

    async fn delete(&self, id: &BirthdayId) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM birthdays WHERE id = ?")
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
        // Fetch all birthdays for user and filter in Rust (SQLite date functions are limited)
        let all = self.find_all_for_user(user_id).await?;
        let today = chrono::Local::now().date_naive();

        Ok(all
            .into_iter()
            .filter(|b| {
                // Calculate next occurrence, handling Feb 29th by falling back to Feb 28th
                let mut next_birthday = b.birth_date.with_year(today.year()).unwrap_or_else(|| {
                    chrono::NaiveDate::from_ymd_opt(today.year(), 2, 28).unwrap()
                });

                if next_birthday < today {
                    next_birthday = b.birth_date.with_year(today.year() + 1).unwrap_or_else(|| {
                        chrono::NaiveDate::from_ymd_opt(today.year() + 1, 2, 28).unwrap()
                    });
                }

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
        .bind(birthday_id.0)
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
            "DELETE FROM reminder_log WHERE reminded_at < datetime('now', '-' || ? || ' days')",
        )
        .bind(older_than_days as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(result.rows_affected())
    }
}
