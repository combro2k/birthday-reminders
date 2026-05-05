use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::channels::domain::repository::{
    NotificationChannelRecord, NotificationChannelRepository,
};
use crate::infrastructure::error::RepositoryError;
use crate::users::domain::user::UserId;

pub struct SqliteNotificationRepo {
    pool: SqlitePool,
}

impl SqliteNotificationRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ChannelRow {
    id: String,
    user_id: String,
    channel_type: String,
    enabled: bool,
    config: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<ChannelRow> for NotificationChannelRecord {
    type Error = RepositoryError;

    fn try_from(row: ChannelRow) -> Result<Self, Self::Error> {
        Ok(NotificationChannelRecord {
            id: Uuid::parse_str(&row.id).map_err(|e| RepositoryError::Database(e.to_string()))?,
            user_id: UserId(
                Uuid::parse_str(&row.user_id)
                    .map_err(|e| RepositoryError::Database(e.to_string()))?,
            ),
            channel_type: row.channel_type,
            enabled: row.enabled,
            config: serde_json::from_str(&row.config)
                .map_err(|e| RepositoryError::Database(e.to_string()))?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }
}

#[async_trait]
impl NotificationChannelRepository for SqliteNotificationRepo {
    async fn find_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<NotificationChannelRecord>, RepositoryError> {
        let rows = sqlx::query_as::<_, ChannelRow>(
            "SELECT id, user_id, channel_type, enabled, config, created_at, updated_at
             FROM notification_channels WHERE user_id = ? ORDER BY channel_type",
        )
        .bind(user_id.0.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn find_by_type(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<Option<NotificationChannelRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, ChannelRow>(
            "SELECT id, user_id, channel_type, enabled, config, created_at, updated_at
             FROM notification_channels WHERE user_id = ? AND channel_type = ?",
        )
        .bind(user_id.0.to_string())
        .bind(channel_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn upsert(
        &self,
        user_id: &UserId,
        channel_type: &str,
        enabled: bool,
        config: serde_json::Value,
    ) -> Result<NotificationChannelRecord, RepositoryError> {
        let id = Uuid::new_v4().to_string();
        let config_str =
            serde_json::to_string(&config).map_err(|e| RepositoryError::Database(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO notification_channels (id, user_id, channel_type, enabled, config, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (user_id, channel_type)
             DO UPDATE SET enabled = excluded.enabled, config = excluded.config, updated_at = excluded.updated_at",
        )
        .bind(&id)
        .bind(user_id.0.to_string())
        .bind(channel_type)
        .bind(enabled)
        .bind(&config_str)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        // Fetch the record (might be the existing one if it was an update)
        self.find_by_type(user_id, channel_type)
            .await?
            .ok_or(RepositoryError::Database("Upsert failed".to_string()))
    }

    async fn delete(&self, user_id: &UserId, channel_type: &str) -> Result<(), RepositoryError> {
        let result =
            sqlx::query("DELETE FROM notification_channels WHERE user_id = ? AND channel_type = ?")
                .bind(user_id.0.to_string())
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
             FROM notification_channels WHERE user_id = ? AND enabled = 1",
        )
        .bind(user_id.0.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}
