use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::error::RepositoryError;
use crate::users::domain::repository::{NewUser, UpdateUser, UserRepository};
use crate::users::domain::user::{AuthMethod, Role, Theme, User, UserId};

pub struct PgUserRepo {
    pool: PgPool,
}

impl PgUserRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    email: String,
    password_hash: Option<String>,
    role: String,
    auth_method: String,
    oidc_subject: Option<String>,
    date_format: String,
    theme: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            id: UserId(row.id),
            username: row.username,
            email: row.email,
            password_hash: row.password_hash,
            role: Role::from_str(&row.role),
            auth_method: AuthMethod::from_str(&row.auth_method),
            oidc_subject: row.oidc_subject,
            date_format: row.date_format,
            theme: Theme::from_str(&row.theme),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl UserRepository for PgUserRepo {
    async fn create(&self, new: NewUser) -> Result<User, RepositoryError> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, UserRow>(
            "INSERT INTO users (id, username, email, password_hash, role, auth_method, oidc_subject)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id, username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, created_at, updated_at",
        )
        .bind(id)
        .bind(&new.username)
        .bind(&new.email)
        .bind(&new.password_hash)
        .bind(new.role.as_str())
        .bind(new.auth_method.as_str())
        .bind(&new.oidc_subject)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") || e.to_string().contains("unique") {
                RepositoryError::AlreadyExists
            } else {
                RepositoryError::Database(e.to_string())
            }
        })?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<User, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, created_at, updated_at
             FROM users WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;
        Ok(row.into())
    }

    async fn find_by_username(&self, username: &str) -> Result<User, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, created_at, updated_at
             FROM users WHERE username = $1",
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
            "SELECT id, username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, created_at, updated_at
             FROM users WHERE oidc_subject = $1",
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
            "SELECT id, username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, created_at, updated_at
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

        let row = sqlx::query_as::<_, UserRow>(
            "UPDATE users SET username = $1, email = $2, password_hash = $3, role = $4,
             auth_method = $5, oidc_subject = $6, date_format = $7, theme = $8, updated_at = NOW()
               WHERE id = $9
             RETURNING id, username, email, password_hash, role, auth_method, oidc_subject, date_format, theme, created_at, updated_at",
        )
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(role.as_str())
        .bind(auth_method.as_str())
        .bind(&oidc_subject)
        .bind(&date_format)
        .bind(theme.as_str())
        .bind(id.0)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(row.into())
    }

    async fn delete(&self, id: &UserId) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id.0)
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
        let row = sqlx::query_scalar::<_, Vec<i32>>(
            "SELECT days_before FROM user_reminder_settings WHERE user_id = $1",
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(row)
    }

    async fn set_reminder_days(
        &self,
        user_id: &UserId,
        days: Vec<i32>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO user_reminder_settings (user_id, days_before) VALUES ($1, $2)
             ON CONFLICT (user_id) DO UPDATE SET days_before = $2",
        )
        .bind(user_id.0)
        .bind(&days)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }
}
