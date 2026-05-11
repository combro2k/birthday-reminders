mod auth;
mod birthdays;
mod channels;
mod cli;
mod infrastructure;
mod reminders;
mod users;

use std::path::Path;
use std::sync::Arc;

use axum::http::header;
use clap::Parser;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::EnvFilter;

use auth::application::auth_service::AuthService;
use auth::infrastructure::oidc::OidcClient;
use birthdays::application::commands::BirthdayCommandService;
use birthdays::application::queries::BirthdayQueryService;
use channels::application::commands::NotificationCommandService;
use channels::infrastructure::signal::{SignalRuntimeConfig, SignalTransport};
use cli::commands::{Cli, Commands};
use infrastructure::config::AppConfig;
use infrastructure::config::SignalTransportMode;
use infrastructure::database::{DatabasePool, Repositories};
use infrastructure::web::server::{self, AppState};
use reminders::application::reminder_job::ReminderJobService;
use users::application::commands::UserCommandService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load config
    let config = AppConfig::load(Path::new(&cli.config))?;

    // Initialize logging based on config
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    if config.logging.output == "syslog" {
        let syslog_writer = infrastructure::logging::SyslogMakeWriter::new()?;
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(syslog_writer)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

    // Drop privileges if running as root and configured (before DB connection)
    #[cfg(unix)]
    {
        use nix::unistd::{Gid, Uid, setgid, setuid};
        use std::process;
        if Uid::effective().is_root() {
            let user = &config.server.run_as_user;
            let group = &config.server.run_as_group;
            match unix_users::get_user_by_name(user) {
                Some(u) => {
                    let target_uid = Uid::from_raw(u.uid());
                    let target_gid = unix_users::get_group_by_name(group)
                        .map(|g| Gid::from_raw(g.gid()))
                        .unwrap_or(Gid::from_raw(u.primary_group_id()));
                    if let Err(e) = setgid(target_gid) {
                        eprintln!("Failed to setgid: {e}");
                        process::exit(1);
                    }
                    if let Err(e) = setuid(target_uid) {
                        eprintln!("Failed to setuid: {e}");
                        process::exit(1);
                    }
                    println!(
                        "Dropped privileges to user '{}' (uid={}, gid={})",
                        user, target_uid, target_gid
                    );
                }
                None => {
                    eprintln!("Configured run_as_user '{}' not found", user);
                    process::exit(1);
                }
            }
        }
    }

    // Create database pool (auto-detects SQLite, MySQL, or PostgreSQL from URL)
    let db = DatabasePool::connect(&config.database.url, config.database.max_connections).await?;

    // Run migrations
    db.run_migrations(cli.debug).await?;

    // Create repositories
    let repos = Repositories::new(&db);
    let user_repo = repos.user_repo;
    let birthday_repo = repos.birthday_repo;
    let notification_repo = repos.notification_repo;

    let signal_runtime = SignalRuntimeConfig {
        transport: match config.commands.signal_transport {
            SignalTransportMode::Cli => SignalTransport::Cli {
                binary_path: config.commands.signal_cli_path.clone(),
            },
            SignalTransportMode::Api => SignalTransport::Api {
                base_url: config.commands.signal_api_url.clone(),
            },
        },
    };

    // Create services
    let user_cmd_svc = UserCommandService::new(user_repo.clone());
    let birthday_cmd_svc = BirthdayCommandService::new(birthday_repo.clone());
    let birthday_query_svc = BirthdayQueryService::new(birthday_repo.clone());
    let notification_svc = NotificationCommandService::new(
        notification_repo.clone(),
        config.server.encryption_key.clone(),
        signal_runtime.clone(),
    );

    let reminder_svc = Arc::new(ReminderJobService::new(
        user_repo.clone(),
        birthday_repo.clone(),
        notification_repo.clone(),
        config.reminders.default_days_before.clone(),
        config.server.encryption_key.clone(),
        signal_runtime,
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
                match config.server.oidc_callback_url() {
                    Ok(callback_url) => match OidcClient::new(oidc_config, &callback_url).await {
                        Ok(client) => {
                            tracing::info!(
                                "OIDC configured with provider: {} ({})",
                                oidc_config.provider_name,
                                callback_url
                            );
                            Some(Arc::new(client))
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to initialize OIDC: {}. Continuing without OIDC.",
                                e
                            );
                            None
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Failed to resolve OIDC callback URL: {}. Continuing without OIDC.",
                            e
                        );
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

        // Bootstrap admin user if none exist in the database
        auth_service.bootstrap_admin_user().await?;

        let state = Arc::new(AppState {
            db: db.clone(),
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

        let compression_layer = CompressionLayer::new().compress_when(
            |_status: axum::http::StatusCode,
             _version: axum::http::Version,
             headers: &axum::http::HeaderMap,
             _extensions: &axum::http::Extensions| {
                // Only compress if the Content-Type header matches our criteria
                headers
                    .get(header::CONTENT_TYPE)
                    .and_then(|val| val.to_str().ok())
                    .map(|mime| {
                        mime.starts_with("text/")
                            || mime.starts_with("application/javascript")
                            || mime.starts_with("application/json")
                            || mime.starts_with("image/svg+xml")
                            || mime.starts_with("application/wasm")
                            || mime.starts_with("application/manifest+json")
                    })
                    .unwrap_or(false)
            },
        );

        let router = server::create_router(state, db.clone())
            .await?
            .layer(compression_layer);

        tracing::info!("Starting server on {}", listen);
        let listener = tokio::net::TcpListener::bind(&listen).await?;
        axum::serve(listener, router).await?;
    } else {
        cli::handlers::handle_command(
            cli.command,
            &db,
            &user_repo,
            &birthday_repo,
            &user_cmd_svc,
            &birthday_cmd_svc,
            &birthday_query_svc,
            &reminder_svc,
            &user_repo,
        )
        .await?;
    }

    Ok(())
}
