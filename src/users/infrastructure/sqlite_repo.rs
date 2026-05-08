use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::infrastructure::error::RepositoryError;
use crate::users::domain::repository::{NewUser, UpdateUser, UserRepository};
use crate::users::domain::user::{AuthMethod, Role, Theme, User, UserId};

pub struct SqliteUserRepo {
    pool: SqlitePool,
}

impl SqliteUserRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    username: String,
    email: String,
    password_hash: Option<String>,
    role: String,
    auth_method: String,
    oidc_subject: Option<String>,
    date_format: String,
    theme: String,
    dashboard_upcoming_days: i64,
    birthday_sort_field: String,
    birthday_sort_desc: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            id: UserId(Uuid::parse_str(&row.id).expect("Invalid UUID in database")),
            username: row.username,
            email: row.email,
            password_hash: row.password_hash,
            role: Role::from_str(&row.role),
            auth_method: AuthMethod::from_str(&row.auth_method),
            oidc_subject: row.oidc_subject,
            date_format: row.date_format,
            theme: Theme::from_str(&row.theme),
            dashboard_upcoming_days: u32::try_from(row.dashboard_upcoming_days).unwrap_or(30),
            birthday_sort_field: row.birthday_sort_field,
            birthday_sort_desc: row.birthday_sort_desc != 0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepo {
    async fn create(&self, new: NewUser) -> Result<User, RepositoryError> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        sqlx::query(
              "INSERT INTO users (id, username, email, password_hash, role, auth_method, oidc_subject, theme, dashboard_upcoming_days, birthday_sort_field, birthday_sort_desc, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&new.username)
        .bind(&new.email)
        .bind(&new.password_hash)
        .bind(new.role.as_str())
        .bind(new.auth_method.as_str())
        .bind(&new.oidc_subject)
        .bind(Theme::default().as_str())
        .bind(30_i64)
        .bind("date")
        .bind(0_i64)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("UNIQUE") || msg.contains("unique") {
                RepositoryError::AlreadyExists
            } else {
                RepositoryError::Database(msg)
            }
        })?;

        Ok(User {
            id: UserId(id),
            username: new.username,
            email: new.email,
            password_hash: new.password_hash,
            role: new.role,
            auth_method: new.auth_method,
            oidc_subject: new.oidc_subject,
            date_format: "%d-%m-%Y".to_string(), // Default date format
            theme: Theme::default(),
            dashboard_upcoming_days: 30,
            birthday_sort_field: "date".to_string(),
            birthday_sort_desc: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    async fn find_by_id(&self, id: &UserId) -> Result<User, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT
                CASE
                    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
                        SUBSTR(HEX(id), 1, 8),
                        SUBSTR(HEX(id), 9, 4),
                        SUBSTR(HEX(id), 13, 4),
                        SUBSTR(HEX(id), 17, 4),
                        SUBSTR(HEX(id), 21, 12)
                    )
                    ELSE id
                END as id,
                     username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, dashboard_upcoming_days, birthday_sort_field, birthday_sort_desc, created_at, updated_at
             FROM users
             WHERE
                CASE
                    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
                        SUBSTR(HEX(id), 1, 8),
                        SUBSTR(HEX(id), 9, 4),
                        SUBSTR(HEX(id), 13, 4),
                        SUBSTR(HEX(id), 17, 4),
                        SUBSTR(HEX(id), 21, 12)
                    )
                    ELSE id
                END = ?",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;
        Ok(row.into())
    }

    async fn find_by_username(&self, username: &str) -> Result<User, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT
                CASE
                    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
                        SUBSTR(HEX(id), 1, 8),
                        SUBSTR(HEX(id), 9, 4),
                        SUBSTR(HEX(id), 13, 4),
                        SUBSTR(HEX(id), 17, 4),
                        SUBSTR(HEX(id), 21, 12)
                    )
                    ELSE id
                END as id,
                     username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, dashboard_upcoming_days, birthday_sort_field, birthday_sort_desc, created_at, updated_at
             FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;
        Ok(row.into())
    }

    async fn find_by_oidc_subject(&self, subject: &str) -> Result<User, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT
                CASE
                    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
                        SUBSTR(HEX(id), 1, 8),
                        SUBSTR(HEX(id), 9, 4),
                        SUBSTR(HEX(id), 13, 4),
                        SUBSTR(HEX(id), 17, 4),
                        SUBSTR(HEX(id), 21, 12)
                    )
                    ELSE id
                END as id,
                     username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, dashboard_upcoming_days, birthday_sort_field, birthday_sort_desc, created_at, updated_at
             FROM users WHERE oidc_subject = ?",
        )
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;
        Ok(row.into())
    }

    async fn find_all(&self) -> Result<Vec<User>, RepositoryError> {
        let rows = sqlx::query_as::<_, UserRow>(
            "SELECT
                CASE
                    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
                        SUBSTR(HEX(id), 1, 8),
                        SUBSTR(HEX(id), 9, 4),
                        SUBSTR(HEX(id), 13, 4),
                        SUBSTR(HEX(id), 17, 4),
                        SUBSTR(HEX(id), 21, 12)
                    )
                    ELSE id
                END as id,
                     username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, dashboard_upcoming_days, birthday_sort_field, birthday_sort_desc, created_at, updated_at
             FROM users ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(&self, id: &UserId, update: UpdateUser) -> Result<User, RepositoryError> {
        let current = self.find_by_id(id).await?;

        let username = update.username.unwrap_or(current.username);
        let email = update.email.unwrap_or(current.email);
        let password_hash = match update.password_hash {
            Some(ph) => ph,
            None => current.password_hash,
        };
        let role = update.role.unwrap_or(current.role);
        let auth_method = update.auth_method.unwrap_or(current.auth_method);
        let oidc_subject = match update.oidc_subject {
            Some(os) => os,
            None => current.oidc_subject,
        };
        let date_format = update.date_format.unwrap_or(current.date_format);
        let theme = update
            .theme
            .map(|t| Theme::from_str(&t))
            .unwrap_or(current.theme);
        let dashboard_upcoming_days = update
            .dashboard_upcoming_days
            .unwrap_or(current.dashboard_upcoming_days);
        let birthday_sort_field = update
            .birthday_sort_field
            .unwrap_or(current.birthday_sort_field);
        let birthday_sort_desc = update
            .birthday_sort_desc
            .unwrap_or(current.birthday_sort_desc);
        let now = chrono::Utc::now();

        sqlx::query(
            "UPDATE users SET username = ?, email = ?, password_hash = ?, role = ?,
             auth_method = ?, oidc_subject = ?, date_format = ?, theme = ?, dashboard_upcoming_days = ?, birthday_sort_field = ?, birthday_sort_desc = ?, updated_at = ?
             WHERE
                CASE
                    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
                        SUBSTR(HEX(id), 1, 8),
                        SUBSTR(HEX(id), 9, 4),
                        SUBSTR(HEX(id), 13, 4),
                        SUBSTR(HEX(id), 17, 4),
                        SUBSTR(HEX(id), 21, 12)
                    )
                    ELSE id
                END = ?",
        )
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(role.as_str())
        .bind(auth_method.as_str())
        .bind(&oidc_subject)
        .bind(&date_format)
        .bind(theme.as_str())
        .bind(dashboard_upcoming_days as i64)
        .bind(&birthday_sort_field)
        .bind(if birthday_sort_desc { 1_i64 } else { 0_i64 })
        .bind(now)
        .bind(id.0.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        self.find_by_id(id).await
    }

    async fn delete(&self, id: &UserId) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM users
             WHERE
                CASE
                    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
                        SUBSTR(HEX(id), 1, 8),
                        SUBSTR(HEX(id), 9, 4),
                        SUBSTR(HEX(id), 13, 4),
                        SUBSTR(HEX(id), 17, 4),
                        SUBSTR(HEX(id), 21, 12)
                    )
                    ELSE id
                END = ?",
        )
        .bind(id.0.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn get_reminder_days(
        &self,
        user_id: &UserId,
    ) -> Result<Option<Vec<i32>>, RepositoryError> {
        let row = sqlx::query_scalar::<_, String>(
            "SELECT days_before FROM user_reminder_settings WHERE user_id = ?",
        )
        .bind(user_id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(row.map(|s| {
            s.split(',')
                .filter_map(|v| v.trim().parse::<i32>().ok())
                .collect()
        }))
    }

    async fn set_reminder_days(
        &self,
        user_id: &UserId,
        days: Vec<i32>,
    ) -> Result<(), RepositoryError> {
        let days_str = days
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(",");

        sqlx::query(
            "INSERT INTO user_reminder_settings (user_id, days_before) VALUES (?, ?)
             ON CONFLICT (user_id) DO UPDATE SET days_before = excluded.days_before",
        )
        .bind(user_id.0.to_string())
        .bind(&days_str)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }
}
