mod application;
mod domain;
mod infrastructure;
mod interface;

use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

use application::auth_service::AuthService;
use application::birthday_commands::BirthdayCommandService;
use application::birthday_queries::BirthdayQueryService;
use application::notification_commands::NotificationCommandService;
use application::reminder_job::ReminderJobService;
use application::user_commands::UserCommandService;
use domain::repository::{BirthdayRepository, NotificationChannelRepository};
use domain::user_repository::UserRepository;
use infrastructure::auth::oidc::OidcClient;
use infrastructure::config::AppConfig;
use infrastructure::persistence::pg_birthday_repo::PgBirthdayRepo;
use infrastructure::persistence::pg_notification_repo::PgNotificationRepo;
use infrastructure::persistence::pg_user_repo::PgUserRepo;
use interface::cli::commands::{Cli, Commands};
use interface::web::server::{self, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cli = Cli::parse();

    // Load config
    let config = AppConfig::load(Path::new(&cli.config))?;

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Create repositories
    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepo::new(pool.clone()));
    let birthday_repo: Arc<dyn BirthdayRepository> = Arc::new(PgBirthdayRepo::new(pool.clone()));
    let notification_repo: Arc<dyn NotificationChannelRepository> =
        Arc::new(PgNotificationRepo::new(pool.clone()));

    // Create services
    let user_cmd_svc = UserCommandService::new(user_repo.clone());
    let birthday_cmd_svc = BirthdayCommandService::new(birthday_repo.clone());
    let birthday_query_svc = BirthdayQueryService::new(birthday_repo.clone());
    let notification_svc = NotificationCommandService::new(
        notification_repo.clone(),
        config.server.session_secret.clone(),
    );

    let reminder_svc = Arc::new(ReminderJobService::new(
        user_repo.clone(),
        birthday_repo.clone(),
        notification_repo.clone(),
        config.reminders.default_days_before.clone(),
        config.server.session_secret.clone(),
    ));

    // Handle commands
    if matches!(&cli.command, Commands::Serve { .. }) {
        let port = match &cli.command {
            Commands::Serve { port } => *port,
            _ => unreachable!(),
        };

        // Initialize OIDC client if configured
        let oidc_client = if let Some(ref oidc_config) = config.auth.oidc {
            if oidc_config.enabled {
                match OidcClient::new(oidc_config, &config.server.base_url).await {
                    Ok(client) => {
                        tracing::info!("OIDC configured with provider: {}", oidc_config.provider_name);
                        Some(Arc::new(client))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize OIDC: {}. Continuing without OIDC.", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let auto_provision = config
            .auth
            .oidc
            .as_ref()
            .map(|o| o.auto_provision)
            .unwrap_or(false);
        let default_role = config
            .auth
            .oidc
            .as_ref()
            .map(|o| o.default_role.clone())
            .unwrap_or_else(|| "user".to_string());

        let auth_service = AuthService::new(
            user_repo.clone(),
            oidc_client.clone(),
            auto_provision,
            default_role,
        );

        let state = Arc::new(AppState {
            pool: pool.clone(),
            config: config.clone(),
            auth_service,
            user_command_service: user_cmd_svc,
            birthday_command_service: birthday_cmd_svc,
            birthday_query_service: birthday_query_svc,
            notification_service: notification_svc,
            user_repo: user_repo.clone(),
            oidc_client,
        });

        // Start scheduler
        let _scheduler = infrastructure::scheduler::start_scheduler(
            &config.reminders.schedule,
            reminder_svc.clone(),
        )
        .await?;

        // Determine listen address
        let listen = if let Some(p) = port {
            format!("0.0.0.0:{}", p)
        } else {
            config.server.listen.clone()
        };

        let router = server::create_router(state, pool).await?;

        tracing::info!("Starting server on {}", listen);
        let listener = tokio::net::TcpListener::bind(&listen).await?;
        axum::serve(listener, router).await?;
    } else {
        interface::cli::handlers::handle_command(
            cli.command,
            &pool,
            &user_repo,
            &birthday_repo,
            &user_cmd_svc,
            &birthday_cmd_svc,
            &birthday_query_svc,
            &reminder_svc,
        )
        .await?;
    }

    Ok(())
}

