use std::sync::Arc;

use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::domain::repository::{BirthdayRepository, NotificationChannelRepository};
use crate::domain::user_repository::UserRepository;
use crate::infrastructure::persistence::pg_birthday_repo::PgBirthdayRepo;
use crate::infrastructure::persistence::pg_notification_repo::PgNotificationRepo;
use crate::infrastructure::persistence::pg_user_repo::PgUserRepo;
use crate::infrastructure::persistence::sqlite_birthday_repo::SqliteBirthdayRepo;
use crate::infrastructure::persistence::sqlite_notification_repo::SqliteNotificationRepo;
use crate::infrastructure::persistence::sqlite_user_repo::SqliteUserRepo;

/// The database pool, which can be either PostgreSQL or SQLite.
#[derive(Clone)]
pub enum DatabasePool {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

impl DatabasePool {
    pub async fn connect(url: &str, max_connections: u32) -> anyhow::Result<Self> {
        if url.starts_with("sqlite") {
            // Extract path from sqlite:// URL
            let path = url
                .strip_prefix("sqlite://")
                .or_else(|| url.strip_prefix("sqlite:"))
                .unwrap_or(url);

            // Create parent directories if needed
            if let Some(parent) = std::path::Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            let pool = SqlitePoolOptions::new()
                .max_connections(max_connections)
                .connect(url)
                .await?;

            // Enable WAL mode and foreign keys
            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&pool)
                .await?;
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&pool)
                .await?;

            Ok(Self::Sqlite(pool))
        } else {
            let pool = PgPoolOptions::new()
                .max_connections(max_connections)
                .connect(url)
                .await?;
            Ok(Self::Postgres(pool))
        }
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        match self {
            Self::Postgres(pool) => {
                sqlx::migrate!("./migrations").run(pool).await?;
            }
            Self::Sqlite(pool) => {
                // Run SQLite-specific migrations manually
                let sql = include_str!("../../migrations/sqlite/001_init.sql");
                sqlx::query(sql).execute(pool).await?;
            }
        }
        Ok(())
    }
}

pub struct Repositories {
    pub user_repo: Arc<dyn UserRepository>,
    pub birthday_repo: Arc<dyn BirthdayRepository>,
    pub notification_repo: Arc<dyn NotificationChannelRepository>,
}

impl Repositories {
    pub fn new(pool: &DatabasePool) -> Self {
        match pool {
            DatabasePool::Postgres(pg) => Self {
                user_repo: Arc::new(PgUserRepo::new(pg.clone())),
                birthday_repo: Arc::new(PgBirthdayRepo::new(pg.clone())),
                notification_repo: Arc::new(PgNotificationRepo::new(pg.clone())),
            },
            DatabasePool::Sqlite(sqlite) => Self {
                user_repo: Arc::new(SqliteUserRepo::new(sqlite.clone())),
                birthday_repo: Arc::new(SqliteBirthdayRepo::new(sqlite.clone())),
                notification_repo: Arc::new(SqliteNotificationRepo::new(sqlite.clone())),
            },
        }
    }
}
