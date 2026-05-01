use async_trait::async_trait;
use chrono::Datelike;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::birthday::{Birthday, BirthdayId};
use crate::domain::repository::{BirthdayRepository, NewBirthday, RepositoryError, UpdateBirthday};
use crate::domain::user::UserId;

pub struct MysqlBirthdayRepo {
    pool: MySqlPool,
}

impl MysqlBirthdayRepo {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BirthdayRow {
    id: String,
    user_id: String,
    name: String,
    birth_date: chrono::NaiveDate,
    phone_number: Option<String>,
    address: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    country: Option<String>,
    notes: Option<String>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
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
            birth_date: row.birth_date,
            phone_number: row.phone_number,
            address: row.address,
            postal_code: row.postal_code,
            city: row.city,
            country: row.country,
            notes: row.notes,
            created_at: chrono::DateTime::from_naive_utc_and_offset(row.created_at, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(row.updated_at, chrono::Utc),
        })
    }
}

#[async_trait]
impl BirthdayRepository for MysqlBirthdayRepo {
    async fn create(&self, new: NewBirthday) -> Result<Birthday, RepositoryError> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().naive_utc();

        sqlx::query(
            "INSERT INTO birthdays (id, user_id, name, birth_date, phone_number, address, postal_code, city, country, notes, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(new.user_id.0.to_string())
        .bind(&new.name)
        .bind(new.birth_date)
        .bind(&new.phone_number)
        .bind(&new.address)
        .bind(&new.postal_code)
        .bind(&new.city)
        .bind(&new.country)
        .bind(&new.notes)
        .bind(now)
        .bind(now)
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
            created_at: chrono::DateTime::from_naive_utc_and_offset(now, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(now, chrono::Utc),
        })
    }

    async fn find_by_id(&self, id: &BirthdayId) -> Result<Birthday, RepositoryError> {
        let row = sqlx::query_as::<_, BirthdayRow>(
            "SELECT id, user_id, name, birth_date, phone_number, address, postal_code, city, country, notes, created_at, updated_at
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
            "SELECT id, user_id, name, birth_date, phone_number, address, postal_code, city, country, notes, created_at, updated_at
             FROM birthdays
             WHERE user_id = ?
             ORDER BY MONTH(birth_date), DAY(birth_date)",
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

        sqlx::query(
            "UPDATE birthdays SET name = ?, birth_date = ?, phone_number = ?, address = ?, postal_code = ?, city = ?, country = ?, notes = ?, updated_at = UTC_TIMESTAMP(6) WHERE id = ?",
        )
        .bind(&name)
        .bind(birth_date)
        .bind(&phone_number)
        .bind(&address)
        .bind(&postal_code)
        .bind(&city)
        .bind(&country)
        .bind(&notes)
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
        let all = self.find_all_for_user(user_id).await?;
        let today = chrono::Local::now().date_naive();

        Ok(all
            .into_iter()
            .filter(|b| {
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
        let count = sqlx::query_scalar::<_, i64>(
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
            "INSERT IGNORE INTO reminder_log (birthday_id, channel_type, days_before, year)
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
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);

        let result = sqlx::query("DELETE FROM reminder_log WHERE reminded_at < ?")
            .bind(cutoff.naive_utc())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }
}
