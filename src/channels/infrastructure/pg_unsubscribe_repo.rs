use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::channels::domain::unsubscribe_token::UnsubscribeToken;
use crate::channels::domain::unsubscribe_token_repository::UnsubscribeTokenRepository;
use crate::infrastructure::error::RepositoryError;
use crate::users::domain::user::UserId;

pub struct PgUnsubscribeTokenRepo {
    pool: PgPool,
}

impl PgUnsubscribeTokenRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    id: Uuid,
    user_id: Uuid,
    channel_type: String,
    token: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<TokenRow> for UnsubscribeToken {
    fn from(row: TokenRow) -> Self {
        UnsubscribeToken {
            id: row.id,
            user_id: UserId(row.user_id),
            channel_type: row.channel_type,
            token: row.token,
            created_at: row.created_at,
            expires_at: row.expires_at,
            used_at: row.used_at,
        }
    }
}

#[async_trait]
impl UnsubscribeTokenRepository for PgUnsubscribeTokenRepo {
    async fn find_by_token(
        &self,
        token: &str,
    ) -> Result<Option<UnsubscribeToken>, RepositoryError> {
        let row = sqlx::query_as::<_, TokenRow>(
            "SELECT id, user_id, channel_type, token, created_at, expires_at, used_at
             FROM unsubscribe_tokens WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(row.map(Into::into))
    }

    async fn find_active_for_user_channel(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<Option<UnsubscribeToken>, RepositoryError> {
        let row = sqlx::query_as::<_, TokenRow>(
            "SELECT id, user_id, channel_type, token, created_at, expires_at, used_at
             FROM unsubscribe_tokens
             WHERE user_id = $1 AND channel_type = $2 AND used_at IS NULL
             AND (expires_at IS NULL OR expires_at > now())",
        )
        .bind(user_id.0)
        .bind(channel_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(row.map(Into::into))
    }

    async fn create(&self, token: &UnsubscribeToken) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO unsubscribe_tokens (id, user_id, channel_type, token, created_at, expires_at, used_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(token.id)
        .bind(token.user_id.0)
        .bind(&token.channel_type)
        .bind(&token.token)
        .bind(token.created_at)
        .bind(token.expires_at)
        .bind(token.used_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }

    async fn mark_used(&self, token: &str) -> Result<(), RepositoryError> {
        let result = sqlx::query("UPDATE unsubscribe_tokens SET used_at = now() WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn delete_for_user_channel(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM unsubscribe_tokens WHERE user_id = $1 AND channel_type = $2")
            .bind(user_id.0)
            .bind(channel_type)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }
}
