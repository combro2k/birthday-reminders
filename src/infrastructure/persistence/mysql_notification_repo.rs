use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::repository::{
    NotificationChannelRecord, NotificationChannelRepository, RepositoryError,
};
use crate::domain::user::UserId;

pub struct MysqlNotificationRepo {
    pool: MySqlPool,
}

impl MysqlNotificationRepo {
    pub fn new(pool: MySqlPool) -> Self {
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
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
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
            created_at: chrono::DateTime::from_naive_utc_and_offset(row.created_at, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(row.updated_at, chrono::Utc),
        })
    }
}

#[async_trait]
impl NotificationChannelRepository for MysqlNotificationRepo {
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
        let user_id_str = user_id.0.to_string();
        let config_str =
            serde_json::to_string(&config).map_err(|e| RepositoryError::Database(e.to_string()))?;
        let now = chrono::Utc::now().naive_utc();

        sqlx::query(
            "INSERT INTO notification_channels (id, user_id, channel_type, enabled, config, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE enabled = VALUES(enabled), config = VALUES(config), updated_at = VALUES(updated_at)",
        )
        .bind(&id)
        .bind(&user_id_str)
        .bind(channel_type)
        .bind(enabled)
        .bind(&config_str)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

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
             FROM notification_channels WHERE user_id = ? AND enabled = TRUE",
        )
        .bind(user_id.0.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}
