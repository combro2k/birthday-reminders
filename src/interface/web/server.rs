use std::sync::Arc;

use axum::{
    extract::State,
    middleware,
    routing::{get, post},
    Router,
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use tower_http::services::ServeDir;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::PostgresStore;

use crate::application::auth_service::AuthService;
use crate::application::birthday_commands::BirthdayCommandService;
use crate::application::birthday_queries::BirthdayQueryService;
use crate::application::notification_commands::NotificationCommandService;
use crate::application::user_commands::UserCommandService;
use crate::domain::user_repository::UserRepository;
use crate::infrastructure::auth::oidc::OidcClient;
use crate::infrastructure::config::AppConfig;

use super::handlers::{admin, auth, birthdays, notifications, settings};
use super::middleware::{auth_middleware, csrf_middleware, rate_limit_middleware, RateLimiter};

pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
    pub auth_service: AuthService,
    pub user_command_service: UserCommandService,
    pub birthday_command_service: BirthdayCommandService,
    pub birthday_query_service: BirthdayQueryService,
    pub notification_service: NotificationCommandService,
    pub user_repo: Arc<dyn UserRepository>,
    pub oidc_client: Option<Arc<OidcClient>>,
}

pub async fn create_router(state: Arc<AppState>, pool: PgPool) -> anyhow::Result<Router> {
    // Session store
    let session_store = PostgresStore::new(pool.clone());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(state.config.server.base_url.starts_with("https"))
        .with_http_only(true);

    // Rate limiter for auth routes: max 5 requests per 60 seconds per IP
    let auth_rate_limiter = Arc::new(RateLimiter::new(5, 60));

    // Public routes (no auth required)
    let auth_routes = Router::new()
        .route("/auth/login", get(auth::login_page).post(auth::login_submit))
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
            rate_limit_middleware(limiter, req, next)
        }));

    let public = Router::new()
        .route("/health", get(health_check))
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
        .route("/settings/profile", get(settings::profile_page))
        .route("/settings/password", post(settings::update_password))
        .route("/settings/reminders", post(settings::update_reminder_days))
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

    let app = Router::new()
        .merge(public)
        .merge(protected)
        .nest_service("/static", ServeDir::new(&state.config.server.static_dir))
        .layer(session_layer)
        .with_state(state);

    Ok(app)
}

async fn health_check(State(state): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
    // Verify DB connectivity
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "unhealthy", "reason": "database unreachable"})),
        ),
    }
}
