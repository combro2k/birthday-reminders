use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::channels::domain::unsubscribe_token::UnsubscribeToken;
use crate::channels::domain::unsubscribe_token_repository::UnsubscribeTokenRepository;
use crate::infrastructure::error::RepositoryError;
use crate::users::domain::user::UserId;

pub struct SqliteUnsubscribeTokenRepo {
    pool: SqlitePool,
}

impl SqliteUnsubscribeTokenRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    id: String,
    user_id: String,
    channel_type: String,
    token: String,
    created_at: String,
    expires_at: Option<String>,
    used_at: Option<String>,
}

impl TryFrom<TokenRow> for UnsubscribeToken {
    type Error = RepositoryError;

    fn try_from(row: TokenRow) -> Result<Self, Self::Error> {
        Ok(UnsubscribeToken {
            id: Uuid::parse_str(&row.id).map_err(|e| RepositoryError::Database(e.to_string()))?,
            user_id: UserId(
                Uuid::parse_str(&row.user_id)
                    .map_err(|e| RepositoryError::Database(e.to_string()))?,
            ),
            channel_type: row.channel_type,
            token: row.token,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            expires_at: row.expires_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            }),
            used_at: row.used_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            }),
        })
    }
}

#[async_trait]
impl UnsubscribeTokenRepository for SqliteUnsubscribeTokenRepo {
    async fn find_by_token(
        &self,
        token: &str,
    ) -> Result<Option<UnsubscribeToken>, RepositoryError> {
        let row = sqlx::query_as::<_, TokenRow>(
            "SELECT id, user_id, channel_type, token, created_at, expires_at, used_at
             FROM unsubscribe_tokens WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_active_for_user_channel(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<Option<UnsubscribeToken>, RepositoryError> {
        let row = sqlx::query_as::<_, TokenRow>(
            "SELECT id, user_id, channel_type, token, created_at, expires_at, used_at
             FROM unsubscribe_tokens
             WHERE user_id = ? AND channel_type = ? AND used_at IS NULL
             AND (expires_at IS NULL OR expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(user_id.0.to_string())
        .bind(channel_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn create(&self, token: &UnsubscribeToken) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO unsubscribe_tokens (id, user_id, channel_type, token, created_at, expires_at, used_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(token.id.to_string())
        .bind(token.user_id.0.to_string())
        .bind(&token.channel_type)
        .bind(&token.token)
        .bind(token.created_at.to_rfc3339())
        .bind(token.expires_at.map(|dt| dt.to_rfc3339()))
        .bind(token.used_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }

    async fn mark_used(&self, token: &str) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE unsubscribe_tokens SET used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE token = ?",
        )
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
        sqlx::query("DELETE FROM unsubscribe_tokens WHERE user_id = ? AND channel_type = ?")
            .bind(user_id.0.to_string())
            .bind(channel_type)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }
}
