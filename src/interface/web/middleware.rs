use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

use crate::domain::user::{User, UserId};
use crate::domain::user_repository::UserRepository;
use crate::infrastructure::auth::{api_token, session};

use super::server::AppState;

/// Extract authenticated user from session or API token
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Response {
    // Try session first
    if let Some(user_id) = session::get_user_id(&session).await {
        match state.user_repo.find_by_id(&user_id).await {
            Ok(user) => {
                request.extensions_mut().insert(user);
                return next.run(request).await;
            }
            Err(_) => {
                session::clear_session(&session).await;
            }
        }
    }

    // Try API token from Authorization header
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                if let Some(user) = validate_bearer_token(token, &state).await {
                    request.extensions_mut().insert(user);
                    return next.run(request).await;
                }
            }
        }
    }

    // Check if this is an API request or browser request
    let is_api = request
        .headers()
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/json"));

    if is_api {
        StatusCode::UNAUTHORIZED.into_response()
    } else {
        Redirect::to("/auth/login").into_response()
    }
}

async fn validate_bearer_token(token: &str, state: &AppState) -> Option<User> {
    let token_hash = api_token::hash_token(token);

    #[derive(sqlx::FromRow)]
    struct TokenLookup {
        user_id: uuid::Uuid,
    }

    let result = sqlx::query_as::<_, TokenLookup>(
        "UPDATE api_tokens SET last_used_at = NOW()
         WHERE token_hash = $1
         RETURNING user_id",
    )
    .bind(&token_hash)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()?;

    let user_id = UserId(result.user_id);
    state.user_repo.find_by_id(&user_id).await.ok()
}

/// Extract the current user from request extensions
pub fn get_current_user(request: &Request) -> Option<&User> {
    request.extensions().get::<User>()
}
