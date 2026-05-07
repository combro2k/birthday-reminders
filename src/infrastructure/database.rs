use std::sync::Arc;

use sqlx::MySqlPool;
use sqlx::PgPool;
use sqlx::SqlitePool;
use sqlx::migrate;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;

use crate::birthdays::domain::repository::BirthdayRepository;
use crate::birthdays::infrastructure::mysql_repo::MysqlBirthdayRepo;
use crate::birthdays::infrastructure::pg_repo::PgBirthdayRepo;
use crate::birthdays::infrastructure::sqlite_repo::SqliteBirthdayRepo;
use crate::channels::domain::repository::NotificationChannelRepository;
use crate::channels::infrastructure::mysql_repo::MysqlNotificationRepo;
use crate::channels::infrastructure::pg_repo::PgNotificationRepo;
use crate::channels::infrastructure::sqlite_repo::SqliteNotificationRepo;
use crate::users::domain::repository::UserRepository;
use crate::users::infrastructure::mysql_repo::MysqlUserRepo;
use crate::users::infrastructure::pg_repo::PgUserRepo;
use crate::users::infrastructure::sqlite_repo::SqliteUserRepo;

/// The database pool, which can be either PostgreSQL or SQLite.
#[derive(Clone)]
pub enum DatabasePool {
    Postgres(PgPool),
    Sqlite(SqlitePool),
    Mysql(MySqlPool),
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
            if let Some(parent) = std::path::Path::new(path).parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)?;
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
        } else if url.starts_with("mysql") {
            let pool = MySqlPoolOptions::new()
                .max_connections(max_connections)
                .connect(url)
                .await?;
            Ok(Self::Mysql(pool))
        } else {
            let pool = PgPoolOptions::new()
                .max_connections(max_connections)
                .connect(url)
                .await?;
            Ok(Self::Postgres(pool))
        }
    }

    pub async fn run_migrations(&self, debug: bool) -> anyhow::Result<()> {
        // Compile-time embedded backend-specific migrations.
        let postgres_migrator = migrate!("./migrations/postgres");
        let sqlite_migrator = migrate!("./migrations/sqlite");
        let mysql_migrator = migrate!("./migrations/mysql");

        let (migrator, backend_name) = match self {
            Self::Postgres(_) => (&postgres_migrator, "postgres"),
            Self::Sqlite(_) => (&sqlite_migrator, "sqlite"),
            Self::Mysql(_) => (&mysql_migrator, "mysql"),
        };

        if debug {
            for migration in migrator.iter() {
                eprintln!(
                    "[DEBUG] Starting migration file: {}_{}.sql",
                    migration.version, migration.description
                );
            }
            eprintln!("[DEBUG] Starting database migrations for backend: {backend_name}");
        }

        let result = match self {
            Self::Postgres(pool) => migrator.run(pool).await,
            Self::Sqlite(pool) => migrator.run(pool).await,
            Self::Mysql(pool) => migrator.run(pool).await,
        };

        match result {
            Ok(_) => {
                if debug {
                    eprintln!("[DEBUG] Database migrations completed successfully.");
                }
            }
            Err(ref e) => eprintln!("[ERROR] Database migration failed: {e}"),
        }
        result.map_err(|e| anyhow::anyhow!(e))
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
            DatabasePool::Mysql(mysql) => Self {
                user_repo: Arc::new(MysqlUserRepo::new(mysql.clone())),
                birthday_repo: Arc::new(MysqlBirthdayRepo::new(mysql.clone())),
                notification_repo: Arc::new(MysqlNotificationRepo::new(mysql.clone())),
            },
        }
    }
}
