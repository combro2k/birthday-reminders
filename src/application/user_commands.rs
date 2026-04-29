use std::sync::Arc;

use crate::domain::user::{AuthMethod, Role, User, UserId};
use crate::domain::user_repository::{NewUser, UpdateUser, UserRepository};
use crate::infrastructure::auth::{api_token, password};
use crate::infrastructure::database::DatabasePool;

pub struct UserCommandService {
    user_repo: Arc<dyn UserRepository>,
}

impl UserCommandService {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_input: &str,
        role: Role,
    ) -> anyhow::Result<User> {
        let hash = password::hash_password(password_input)?;
        let new_user = NewUser {
            username: username.to_string(),
            email: email.to_string(),
            password_hash: Some(hash),
            role,
            auth_method: AuthMethod::Local,
            oidc_subject: None,
            date_format: "%d-%m-%Y".to_string(), // Default date format
        };
        let user = self
            .user_repo
            .create(new_user)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create user: {}", e))?;
        Ok(user)
    }

    pub async fn update_password(
        &self,
        user_id: &UserId,
        new_password: &str,
    ) -> anyhow::Result<()> {
        let hash = password::hash_password(new_password)?;
        let update = UpdateUser {
            password_hash: Some(Some(hash)),
            ..Default::default()
        };
        self.user_repo
            .update(user_id, update)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update password: {}", e))?;
        Ok(())
    }

    pub async fn delete_user(&self, user_id: &UserId) -> anyhow::Result<()> {
        self.user_repo
            .delete(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete user: {}", e))?;
        Ok(())
    }

    /// Generate a new API token for a user. Returns (plain_token, token_name).
    /// The plain token is shown once to the user, then stored as hash.
    pub async fn generate_api_token(
        &self,
        user_id: &UserId,
        name: &str,
        db: &DatabasePool,
    ) -> anyhow::Result<String> {
        let (plain, hash) = api_token::generate_api_token();
        let id = uuid::Uuid::new_v4();

        match db {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO api_tokens (id, user_id, token_hash, name) VALUES ($1, $2, $3, $4)",
                )
                .bind(id)
                .bind(user_id.0)
                .bind(&hash)
                .bind(name)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO api_tokens (id, user_id, token_hash, name) VALUES (?, ?, ?, ?)",
                )
                .bind(id.to_string())
                .bind(user_id.0.to_string())
                .bind(&hash)
                .bind(name)
                .execute(pool)
                .await?;
            }
        }

        Ok(plain)
    }

    pub async fn revoke_api_token(
        &self,
        token_id: uuid::Uuid,
        user_id: &UserId,
        db: &DatabasePool,
    ) -> anyhow::Result<()> {
        let rows_affected = match db {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
                    .bind(token_id)
                    .bind(user_id.0)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM api_tokens WHERE id = ? AND user_id = ?")
                    .bind(token_id.to_string())
                    .bind(user_id.0.to_string())
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };
        if rows_affected == 0 {
            anyhow::bail!("Token not found");
        }
        Ok(())
    }

    pub async fn list_api_tokens(
        &self,
        user_id: &UserId,
        db: &DatabasePool,
    ) -> anyhow::Result<Vec<ApiTokenInfo>> {
        match db {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query_as::<_, PgApiTokenRow>(
                    "SELECT id, name, created_at, last_used_at FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
                )
                .bind(user_id.0)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|r| ApiTokenInfo {
                        id: r.id,
                        name: r.name,
                        created_at: r.created_at,
                        last_used_at: r.last_used_at,
                    })
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query_as::<_, SqliteApiTokenRow>(
                    "SELECT id, name, created_at, last_used_at FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC",
                )
                .bind(user_id.0.to_string())
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .filter_map(|r| {
                        Some(ApiTokenInfo {
                            id: uuid::Uuid::parse_str(&r.id).ok()?,
                            name: r.name,
                            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now()),
                            last_used_at: r.last_used_at.and_then(|s| {
                                chrono::DateTime::parse_from_rfc3339(&s)
                                    .map(|dt| dt.with_timezone(&chrono::Utc))
                                    .ok()
                            }),
                        })
                    })
                    .collect())
            }
        }
    }

    /// Resolve an API token to a UserId, updating last_used_at.
    pub async fn resolve_api_token(
        &self,
        token: &str,
        db: &DatabasePool,
    ) -> anyhow::Result<UserId> {
        let token_hash = api_token::hash_token(token);

        match db {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query_scalar::<_, uuid::Uuid>(
                    "UPDATE api_tokens SET last_used_at = NOW() WHERE token_hash = $1 RETURNING user_id",
                )
                .bind(&token_hash)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Invalid API token"))?;

                Ok(UserId(row))
            }
            DatabasePool::Sqlite(pool) => {
                // SQLite doesn't support UPDATE ... RETURNING in older versions,
                // so we do it in two steps
                let user_id_str = sqlx::query_scalar::<_, String>(
                    "SELECT user_id FROM api_tokens WHERE token_hash = ?",
                )
                .bind(&token_hash)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Invalid API token"))?;

                sqlx::query(
                    "UPDATE api_tokens SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE token_hash = ?",
                )
                .bind(&token_hash)
                .execute(pool)
                .await?;

                let uuid = uuid::Uuid::parse_str(&user_id_str)?;
                Ok(UserId(uuid))
            }
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PgApiTokenRow {
    id: uuid::Uuid,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct SqliteApiTokenRow {
    id: String,
    name: String,
    created_at: String,
    last_used_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiTokenInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}
