use std::sync::Arc;

use crate::infrastructure::assets::static_handler;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    middleware,
    response::Html,
    routing::{any_service, get, post},
};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::MySqlStore;
use tower_sessions_sqlx_store::PostgresStore;
use tower_sessions_sqlx_store::SqliteStore;

use crate::auth::application::auth_service::AuthService;
use crate::auth::infrastructure::oidc::OidcClient;
use crate::birthdays::application::commands::BirthdayCommandService;
use crate::birthdays::application::queries::BirthdayQueryService;
use crate::channels::application::commands::NotificationCommandService;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::database::DatabasePool;
use crate::users::application::commands::UserCommandService;
use crate::users::domain::repository::UserRepository;

use super::middleware::{
    ProxyTrust, RateLimiter, auth_middleware, csrf_middleware, rate_limit_middleware,
};
use crate::auth::presentation::handlers as auth;
use crate::birthdays::presentation::handlers as birthdays;
use crate::channels::presentation::handlers as notifications;
use crate::users::presentation::admin;
use crate::users::presentation::settings;

pub struct AppState {
    pub db: DatabasePool,
    pub config: AppConfig,
    pub auth_service: AuthService,
    pub user_command_service: UserCommandService,
    pub birthday_command_service: BirthdayCommandService,
    pub birthday_query_service: BirthdayQueryService,
    pub notification_service: NotificationCommandService,
    pub user_repo: Arc<dyn UserRepository>,
    pub oidc_client: Option<Arc<OidcClient>>,
}

pub async fn create_router(state: Arc<AppState>, db: DatabasePool) -> anyhow::Result<Router> {
    let secure_cookies = state.config.server.secure_cookies()?;
    let proxy_trust = Arc::new(ProxyTrust::new(state.config.server.trusted_proxy_nets()?));

    match &db {
        DatabasePool::Postgres(pool) => {
            let session_store = PostgresStore::new(pool.clone());
            session_store.migrate().await?;
            let session_layer = SessionManagerLayer::new(session_store)
                .with_secure(secure_cookies)
                .with_http_only(true);
            Ok(build_app(state, session_layer, proxy_trust))
        }
        DatabasePool::Sqlite(pool) => {
            let session_store = SqliteStore::new(pool.clone());
            session_store.migrate().await?;
            let session_layer = SessionManagerLayer::new(session_store)
                .with_secure(secure_cookies)
                .with_http_only(true);
            Ok(build_app(state, session_layer, proxy_trust))
        }
        DatabasePool::Mysql(pool) => {
            let session_store = MySqlStore::new(pool.clone());
            session_store.migrate().await?;
            let session_layer = SessionManagerLayer::new(session_store)
                .with_secure(secure_cookies)
                .with_http_only(true);
            Ok(build_app(state, session_layer, proxy_trust))
        }
    }
}

fn build_app<S: tower_sessions::session_store::SessionStore + Clone>(
    state: Arc<AppState>,
    session_layer: SessionManagerLayer<S>,
    proxy_trust: Arc<ProxyTrust>,
) -> Router {
    // Rate limiter for auth routes: max 10 requests per 60 seconds per IP
    let auth_rate_limiter = Arc::new(RateLimiter::new(10, 60));

    // Public routes (no auth required)
    let auth_routes = Router::new()
        .route(
            "/auth/login",
            get(auth::login_page).post(auth::login_submit),
        )
        .route("/auth/logout", post(auth::logout))
        .route(
            "/auth/register",
            get(auth::register_page).post(auth::register_submit),
        )
        .route("/auth/oidc", get(auth::oidc_login))
        .route("/auth/oidc/callback", get(auth::oidc_callback))
        .layer(middleware::from_fn(csrf_middleware))
        .layer(middleware::from_fn(move |req, next| {
            let limiter = auth_rate_limiter.clone();
            let proxy_trust = proxy_trust.clone();
            rate_limit_middleware(limiter, proxy_trust, req, next)
        }));

    let public = Router::new()
        .route("/health", get(health_check))
        .route("/offline", get(offline_page))
        .merge(auth_routes);

    // Protected routes (auth required)
    let protected = Router::new()
        .route("/", get(birthdays::dashboard))
        .route("/birthdays", get(birthdays::list_birthdays))
        .route(
            "/birthdays/new",
            get(birthdays::new_birthday_form).post(birthdays::create_birthday),
        )
        .route(
            "/birthdays/{id}/edit",
            get(birthdays::edit_birthday_form).post(birthdays::update_birthday),
        )
        .route("/birthdays/{id}/delete", post(birthdays::delete_birthday))
        // Notifications
        .route("/notifications", get(notifications::list_channels))
        .route(
            "/notifications/{channel_type}",
            get(notifications::channel_form).post(notifications::save_channel),
        )
        .route(
            "/notifications/{channel_type}/test",
            post(notifications::test_channel),
        )
        .route(
            "/notifications/{channel_type}/delete",
            post(notifications::delete_channel),
        )
        // Settings
        .route(
            "/settings/profile",
            get(settings::profile_page).post(settings::update_date_format),
        )
        .route("/settings/password", post(settings::update_password))
        .route("/settings/reminders", post(settings::update_reminder_days))
        .route("/settings/theme", post(settings::update_theme))
        .route(
            "/settings/dashboard-window",
            post(settings::update_dashboard_window),
        )
        .route(
            "/settings/birthday-sort",
            post(settings::update_birthday_sort_preferences),
        )
        .route(
            "/settings/api-tokens",
            get(settings::api_tokens_page).post(settings::create_api_token),
        )
        .route(
            "/settings/api-tokens/{id}/revoke",
            post(settings::revoke_api_token),
        )
        // Admin
        .route(
            "/admin/users",
            get(admin::users_page).post(admin::create_user),
        )
        .route("/admin/users/{id}/delete", post(admin::delete_user))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn(csrf_middleware));

    let mut app = Router::new()
        .merge(public)
        .merge(protected)
        .route("/static/{*path}", get(static_handler))
        .layer(session_layer)
        .with_state(state.clone());

    if state.config.mcp.enabled {
        let mcp_service =
            crate::mcp::presentation::streamable_http::build_streamable_http_service(state.clone());
        app = app.route_service(
            &state.config.mcp.path,
            any_service(mcp_service).layer(middleware::from_fn_with_state(
                state.clone(),
                crate::mcp::infrastructure::auth::mcp_auth_middleware,
            )),
        );
    }

    app
}

async fn health_check(State(state): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
    // Verify DB connectivity
    let result = match &state.db {
        DatabasePool::Postgres(pool) => {
            sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(pool)
                .await
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(pool)
                .await
        }
        DatabasePool::Mysql(pool) => {
            sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(pool)
                .await
        }
    };
    match result {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "unhealthy", "reason": "database unreachable"})),
        ),
    }
}

async fn offline_page() -> Html<&'static str> {
    Html(include_str!("../../../templates/offline.html"))
}
