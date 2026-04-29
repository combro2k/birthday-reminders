use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::repository::{NotificationChannelRecord, NotificationChannelRepository, RepositoryError};
use crate::domain::user::UserId;

pub struct PgNotificationRepo {
    pool: PgPool,
}

impl PgNotificationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ChannelRow {
    id: Uuid,
    user_id: Uuid,
    channel_type: String,
    enabled: bool,
    config: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<ChannelRow> for NotificationChannelRecord {
    fn from(row: ChannelRow) -> Self {
        NotificationChannelRecord {
            id: row.id,
            user_id: UserId(row.user_id),
            channel_type: row.channel_type,
            enabled: row.enabled,
            config: row.config,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl NotificationChannelRepository for PgNotificationRepo {
    async fn find_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<NotificationChannelRecord>, RepositoryError> {
        let rows = sqlx::query_as::<_, ChannelRow>(
            "SELECT id, user_id, channel_type, enabled, config, created_at, updated_at
             FROM notification_channels WHERE user_id = $1 ORDER BY channel_type",
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_by_type(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<Option<NotificationChannelRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, ChannelRow>(
            "SELECT id, user_id, channel_type, enabled, config, created_at, updated_at
             FROM notification_channels WHERE user_id = $1 AND channel_type = $2",
        )
        .bind(user_id.0)
        .bind(channel_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(row.map(Into::into))
    }

    async fn upsert(
        &self,
        user_id: &UserId,
        channel_type: &str,
        enabled: bool,
        config: serde_json::Value,
    ) -> Result<NotificationChannelRecord, RepositoryError> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, ChannelRow>(
            "INSERT INTO notification_channels (id, user_id, channel_type, enabled, config)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id, channel_type)
             DO UPDATE SET enabled = $4, config = $5, updated_at = NOW()
             RETURNING id, user_id, channel_type, enabled, config, created_at, updated_at",
        )
        .bind(id)
        .bind(user_id.0)
        .bind(channel_type)
        .bind(enabled)
        .bind(&config)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(row.into())
    }

    async fn delete(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM notification_channels WHERE user_id = $1 AND channel_type = $2",
        )
        .bind(user_id.0)
        .bind(channel_type)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn find_enabled_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<NotificationChannelRecord>, RepositoryError> {
        let rows = sqlx::query_as::<_, ChannelRow>(
            "SELECT id, user_id, channel_type, enabled, config, created_at, updated_at
             FROM notification_channels WHERE user_id = $1 AND enabled = true",
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
