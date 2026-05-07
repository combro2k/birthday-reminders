use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use serde::Serialize;

use crate::birthdays::domain::birthday::Birthday;
use crate::birthdays::domain::repository::BirthdayRepository;
use crate::infrastructure::database::DatabasePool;
use crate::users::domain::repository::UserRepository;
use crate::users::domain::user::{User, UserId};

/// Represents a birthday row for CSV export
#[derive(Debug, Serialize)]
pub struct BirthdayExportRow {
    id: String,
    user_id: String,
    name: String,
    birth_date: String,
    email: Option<String>,
    phone_number: Option<String>,
    address: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    country: Option<String>,
    notes: Option<String>,
    created_at: String,
    updated_at: String,
}

impl BirthdayExportRow {
    fn from_birthday(b: Birthday) -> Self {
        Self {
            id: b.id.0.to_string(),
            user_id: b.user_id.0.to_string(),
            name: b.name,
            birth_date: b.birth_date.to_string(),
            email: b.email,
            phone_number: b.phone_number,
            address: b.address,
            postal_code: b.postal_code,
            city: b.city,
            country: b.country,
            notes: b.notes,
            created_at: b.created_at.to_rfc3339(),
            updated_at: b.updated_at.to_rfc3339(),
        }
    }
}

/// Represents a user row for CSV export (admin only)
#[derive(Debug, Serialize)]
pub struct UserExportRow {
    id: String,
    username: String,
    email: String,
    role: String,
    auth_method: String,
    date_format: String,
    theme: String,
    created_at: String,
}

impl UserExportRow {
    fn from_user(u: User) -> Self {
        Self {
            id: u.id.0.to_string(),
            username: u.username,
            email: u.email,
            role: u.role.as_str().to_string(),
            auth_method: u.auth_method.as_str().to_string(),
            date_format: u.date_format,
            theme: u.theme.as_str().to_string(),
            created_at: u.created_at.to_rfc3339(),
        }
    }
}

/// Represents a notification channel row for CSV export
#[derive(Debug, Serialize)]
pub struct NotificationExportRow {
    id: String,
    user_id: String,
    channel_type: String,
    enabled: bool,
    config: String,
    created_at: String,
    updated_at: String,
}

/// Represents a reminder setting row for CSV export
#[derive(Debug, Serialize)]
pub struct ReminderSettingExportRow {
    user_id: String,
    days_before: String,
}

pub struct ExportService {
    birthday_repo: Arc<dyn BirthdayRepository>,
    user_repo: Arc<dyn UserRepository>,
}

impl ExportService {
    pub fn new(
        birthday_repo: Arc<dyn BirthdayRepository>,
        user_repo: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            birthday_repo,
            user_repo,
        }
    }

    /// Export all birthdays for a user
    pub async fn export_birthdays_for_user(&self, user_id: &UserId) -> anyhow::Result<String> {
        let birthdays = self
            .birthday_repo
            .find_all_for_user(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch birthdays: {}", e))?;

        let rows: Vec<BirthdayExportRow> = birthdays
            .into_iter()
            .map(BirthdayExportRow::from_birthday)
            .collect();
        self.write_csv(&rows, "birthdays.csv")
    }

    /// Export all birthdays (admin only)
    pub async fn export_all_birthdays(&self) -> anyhow::Result<String> {
        let users = self
            .user_repo
            .find_all()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch users: {}", e))?;

        let mut all_birthdays = Vec::new();
        for user in users {
            if let Ok(birthdays) = self.birthday_repo.find_all_for_user(&user.id).await {
                all_birthdays.extend(birthdays);
            }
        }

        let rows: Vec<BirthdayExportRow> = all_birthdays
            .into_iter()
            .map(BirthdayExportRow::from_birthday)
            .collect();
        self.write_csv(&rows, "birthdays.csv")
    }

    /// Export all users (admin only)
    pub async fn export_all_users(&self) -> anyhow::Result<String> {
        let users = self
            .user_repo
            .find_all()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch users: {}", e))?;

        let rows: Vec<UserExportRow> = users.into_iter().map(UserExportRow::from_user).collect();
        self.write_csv(&rows, "users.csv")
    }

    /// Export API tokens from database (admin or user's own)
    pub async fn export_api_tokens(
        &self,
        user_id: &UserId,
        db: &DatabasePool,
    ) -> anyhow::Result<String> {
        let rows = self.fetch_api_tokens(user_id, db).await?;
        self.write_csv(&rows, "api_tokens.csv")
    }

    /// Export notification channels from database (admin or user's own)
    pub async fn export_notifications(
        &self,
        user_id: &UserId,
        db: &DatabasePool,
    ) -> anyhow::Result<String> {
        let rows = self.fetch_notifications(user_id, db).await?;
        self.write_csv(&rows, "notifications.csv")
    }

    /// Export reminder settings from database (admin or user's own)
    pub async fn export_reminder_settings(
        &self,
        user_id: &UserId,
        db: &DatabasePool,
    ) -> anyhow::Result<String> {
        let rows = self.fetch_reminder_settings(user_id, db).await?;
        self.write_csv(&rows, "reminders.csv")
    }

    /// Fetch all API tokens for a user
    async fn fetch_api_tokens(
        &self,
        user_id: &UserId,
        db: &DatabasePool,
    ) -> anyhow::Result<Vec<ApiTokenExportRow>> {
        match db {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
                    "SELECT id, user_id, token_hash, name, created_at, last_used_at FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
                )
                .bind(user_id.0)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(
                        |(id, user_id, token_hash, name, created_at, last_used_at)| {
                            ApiTokenExportRow {
                                id,
                                user_id,
                                token_hash,
                                name,
                                created_at,
                                last_used_at,
                            }
                        },
                    )
                    .collect())
            }
            DatabasePool::Mysql(pool) => {
                let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
                    "SELECT id, user_id, token_hash, name, created_at, COALESCE(DATE_FORMAT(last_used_at, '%Y-%m-%dT%H:%i:%sZ'), NULL) FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC",
                )
                .bind(user_id.0.to_string())
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(
                        |(id, user_id, token_hash, name, created_at, last_used_at)| {
                            ApiTokenExportRow {
                                id,
                                user_id,
                                token_hash,
                                name,
                                created_at,
                                last_used_at,
                            }
                        },
                    )
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
                    "SELECT id, user_id, token_hash, name, created_at, last_used_at FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC",
                )
                .bind(user_id.0.to_string())
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(
                        |(id, user_id, token_hash, name, created_at, last_used_at)| {
                            ApiTokenExportRow {
                                id,
                                user_id,
                                token_hash,
                                name,
                                created_at,
                                last_used_at,
                            }
                        },
                    )
                    .collect())
            }
        }
    }

    /// Fetch notification channels for a user
    async fn fetch_notifications(
        &self,
        user_id: &UserId,
        db: &DatabasePool,
    ) -> anyhow::Result<Vec<NotificationExportRow>> {
        match db {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query_as::<_, (String, String, String, bool, String, String, String)>(
                    "SELECT id, user_id, channel_type, enabled, config::text AS config, created_at, updated_at FROM notification_channels WHERE user_id = $1 ORDER BY channel_type",
                )
                .bind(user_id.0)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(
                        |(id, user_id, channel_type, enabled, config, created_at, updated_at)| {
                            NotificationExportRow {
                                id,
                                user_id,
                                channel_type,
                                enabled,
                                config,
                                created_at,
                                updated_at,
                            }
                        },
                    )
                    .collect())
            }
            DatabasePool::Mysql(pool) => {
                let rows = sqlx::query_as::<_, (String, String, String, bool, String, String, String)>(
                    "SELECT id, user_id, channel_type, enabled, config, created_at, updated_at FROM notification_channels WHERE user_id = ? ORDER BY channel_type",
                )
                .bind(user_id.0.to_string())
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(
                        |(id, user_id, channel_type, enabled, config, created_at, updated_at)| {
                            NotificationExportRow {
                                id,
                                user_id,
                                channel_type,
                                enabled,
                                config,
                                created_at,
                                updated_at,
                            }
                        },
                    )
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query_as::<_, (String, String, String, bool, String, String, String)>(
                    "SELECT id, user_id, channel_type, enabled, config, created_at, updated_at FROM notification_channels WHERE user_id = ? ORDER BY channel_type",
                )
                .bind(user_id.0.to_string())
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(
                        |(id, user_id, channel_type, enabled, config, created_at, updated_at)| {
                            NotificationExportRow {
                                id,
                                user_id,
                                channel_type,
                                enabled,
                                config,
                                created_at,
                                updated_at,
                            }
                        },
                    )
                    .collect())
            }
        }
    }

    /// Fetch reminder settings for a user
    async fn fetch_reminder_settings(
        &self,
        user_id: &UserId,
        db: &DatabasePool,
    ) -> anyhow::Result<Vec<ReminderSettingExportRow>> {
        match db {
            DatabasePool::Postgres(pool) => {
                if let Ok(Some(days)) = sqlx::query_scalar::<_, Vec<i32>>(
                    "SELECT days_before FROM user_reminder_settings WHERE user_id = $1",
                )
                .bind(user_id.0)
                .fetch_optional(pool)
                .await
                {
                    let days_str = days
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    Ok(vec![ReminderSettingExportRow {
                        user_id: user_id.0.to_string(),
                        days_before: days_str,
                    }])
                } else {
                    Ok(vec![])
                }
            }
            DatabasePool::Mysql(pool) => {
                if let Ok(Some(days_str)) = sqlx::query_scalar::<_, String>(
                    "SELECT days_before FROM user_reminder_settings WHERE user_id = ?",
                )
                .bind(user_id.0.to_string())
                .fetch_optional(pool)
                .await
                {
                    Ok(vec![ReminderSettingExportRow {
                        user_id: user_id.0.to_string(),
                        days_before: days_str,
                    }])
                } else {
                    Ok(vec![])
                }
            }
            DatabasePool::Sqlite(pool) => {
                if let Ok(Some(days_str)) = sqlx::query_scalar::<_, String>(
                    "SELECT days_before FROM user_reminder_settings WHERE user_id = ?",
                )
                .bind(user_id.0.to_string())
                .fetch_optional(pool)
                .await
                {
                    Ok(vec![ReminderSettingExportRow {
                        user_id: user_id.0.to_string(),
                        days_before: days_str,
                    }])
                } else {
                    Ok(vec![])
                }
            }
        }
    }

    /// Write CSV data using the csv crate
    fn write_csv<T: Serialize>(&self, rows: &[T], _filename: &str) -> anyhow::Result<String> {
        let mut writer = csv::Writer::from_writer(vec![]);

        for row in rows {
            writer.serialize(row)?;
        }

        let bytes = writer.into_inner()?;
        let csv_string = String::from_utf8(bytes)?;
        Ok(csv_string)
    }

    /// Write multiple CSV files to a directory
    pub fn write_csv_files(
        &self,
        data: Vec<(&str, String)>,
        output_dir: &str,
    ) -> anyhow::Result<()> {
        let path = std::path::Path::new(output_dir);
        std::fs::create_dir_all(path)?;

        for (filename, content) in data {
            let file_path = path.join(filename);
            let mut file = std::fs::File::create(&file_path)?;
            file.write_all(content.as_bytes())?;
            println!("Exported to: {}", file_path.display());
        }

        Ok(())
    }

    /// Write single CSV file
    pub fn write_csv_file(&self, content: &str, output_path: &str) -> anyhow::Result<()> {
        let path = Path::new(output_path);
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && parent != Path::new("")
        {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        println!("Exported to: {}", path.display());
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct ApiTokenExportRow {
    id: String,
    user_id: String,
    token_hash: String,
    name: String,
    created_at: String,
    last_used_at: Option<String>,
}
