use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::repository::RepositoryError;
use crate::domain::user::{AuthMethod, Role, User, UserId};
use crate::domain::user_repository::{NewUser, UpdateUser, UserRepository};

pub struct MysqlUserRepo {
    pool: MySqlPool,
}

impl MysqlUserRepo {
    pub fn new(pool: MySqlPool) -> Self {
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
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

impl TryFrom<UserRow> for User {
    type Error = RepositoryError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: UserId(Uuid::parse_str(&row.id).map_err(|e| RepositoryError::Database(e.to_string()))?),
            username: row.username,
            email: row.email,
            password_hash: row.password_hash,
            role: Role::from_str(&row.role),
            auth_method: AuthMethod::from_str(&row.auth_method),
            oidc_subject: row.oidc_subject,
            date_format: row.date_format,
            created_at: chrono::DateTime::from_naive_utc_and_offset(row.created_at, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(row.updated_at, chrono::Utc),
        })
    }
}

#[async_trait]
impl UserRepository for MysqlUserRepo {
    async fn create(&self, new: NewUser) -> Result<User, RepositoryError> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().naive_utc();

        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role, auth_method, oidc_subject, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&new.username)
        .bind(&new.email)
        .bind(&new.password_hash)
        .bind(new.role.as_str())
        .bind(new.auth_method.as_str())
        .bind(&new.oidc_subject)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") || msg.contains("duplicate") || msg.contains("UNIQUE") {
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
            date_format: "%d-%m-%Y".to_string(),
            created_at: chrono::DateTime::from_naive_utc_and_offset(now, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(now, chrono::Utc),
        })
    }

    async fn find_by_id(&self, id: &UserId) -> Result<User, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, email, password_hash, role, auth_method, oidc_subject, date_format, created_at, updated_at
             FROM users WHERE id = ?",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        row.try_into()
    }

    async fn find_by_username(&self, username: &str) -> Result<User, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, email, password_hash, role, auth_method, oidc_subject, date_format, created_at, updated_at
             FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        row.try_into()
    }

    async fn find_by_oidc_subject(&self, subject: &str) -> Result<User, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, email, password_hash, role, auth_method, oidc_subject, date_format, created_at, updated_at
             FROM users WHERE oidc_subject = ?",
        )
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        row.try_into()
    }

    async fn find_all(&self) -> Result<Vec<User>, RepositoryError> {
        let rows = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, email, password_hash, role, auth_method, oidc_subject, date_format, created_at, updated_at
             FROM users ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
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

        sqlx::query(
            "UPDATE users SET username = ?, email = ?, password_hash = ?, role = ?,
             auth_method = ?, oidc_subject = ?, date_format = ?, updated_at = UTC_TIMESTAMP(6)
             WHERE id = ?",
        )
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(role.as_str())
        .bind(auth_method.as_str())
        .bind(&oidc_subject)
        .bind(&date_format)
        .bind(id.0.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        self.find_by_id(id).await
    }

    async fn delete(&self, id: &UserId) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
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
             ON DUPLICATE KEY UPDATE days_before = VALUES(days_before)",
        )
        .bind(user_id.0.to_string())
        .bind(&days_str)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }
}
