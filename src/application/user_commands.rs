use std::sync::Arc;

use crate::domain::user::{AuthMethod, Role, User, UserId};
use crate::domain::user_repository::{NewUser, UpdateUser, UserRepository};
use crate::infrastructure::auth::{api_token, password};

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
        };
        let user = self.user_repo.create(new_user).await.map_err(|e| {
            anyhow::anyhow!("Failed to create user: {}", e)
        })?;
        Ok(user)
    }

    pub async fn update_password(&self, user_id: &UserId, new_password: &str) -> anyhow::Result<()> {
        let hash = password::hash_password(new_password)?;
        let update = UpdateUser {
            password_hash: Some(Some(hash)),
            ..Default::default()
        };
        self.user_repo.update(user_id, update).await.map_err(|e| {
            anyhow::anyhow!("Failed to update password: {}", e)
        })?;
        Ok(())
    }

    pub async fn update_role(&self, user_id: &UserId, role: Role) -> anyhow::Result<User> {
        let update = UpdateUser {
            role: Some(role),
            ..Default::default()
        };
        let user = self.user_repo.update(user_id, update).await.map_err(|e| {
            anyhow::anyhow!("Failed to update role: {}", e)
        })?;
        Ok(user)
    }

    pub async fn delete_user(&self, user_id: &UserId) -> anyhow::Result<()> {
        self.user_repo.delete(user_id).await.map_err(|e| {
            anyhow::anyhow!("Failed to delete user: {}", e)
        })?;
        Ok(())
    }

    /// Generate a new API token for a user. Returns (plain_token, token_name).
    /// The plain token is shown once to the user, then stored as hash.
    pub async fn generate_api_token(
        &self,
        user_id: &UserId,
        name: &str,
        pool: &sqlx::PgPool,
    ) -> anyhow::Result<String> {
        let (plain, hash) = api_token::generate_api_token();

        sqlx::query(
            "INSERT INTO api_tokens (id, user_id, token_hash, name) VALUES ($1, $2, $3, $4)",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(user_id.0)
        .bind(&hash)
        .bind(name)
        .execute(pool)
        .await?;

        Ok(plain)
    }

    pub async fn revoke_api_token(
        &self,
        token_id: uuid::Uuid,
        user_id: &UserId,
        pool: &sqlx::PgPool,
    ) -> anyhow::Result<()> {
        let result =
            sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
                .bind(token_id)
                .bind(user_id.0)
                .execute(pool)
                .await?;
        if result.rows_affected() == 0 {
            anyhow::bail!("Token not found");
        }
        Ok(())
    }

    pub async fn list_api_tokens(
        &self,
        user_id: &UserId,
        pool: &sqlx::PgPool,
    ) -> anyhow::Result<Vec<ApiTokenInfo>> {
        let rows = sqlx::query_as::<_, ApiTokenRow>(
            "SELECT id, name, created_at, last_used_at FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id.0)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|r| ApiTokenInfo {
            id: r.id,
            name: r.name,
            created_at: r.created_at,
            last_used_at: r.last_used_at,
        }).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ApiTokenRow {
    id: uuid::Uuid,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct ApiTokenInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}
