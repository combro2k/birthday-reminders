use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::infrastructure::web::server::AppState;
use crate::mcp::application::birthdays::HTTP_AUTH_USER_ID;

pub async fn mcp_auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|header| header.to_str().ok());

    if let Some(header_value) = auth_header {
        if let Some(token) = header_value.strip_prefix("Bearer ") {
            let token = token.trim();
            if token.is_empty() {
                return (StatusCode::UNAUTHORIZED, "Invalid API token").into_response();
            }

            return match state
                .user_command_service
                .resolve_api_token(token, &state.db)
                .await
            {
                Ok(user_id) => HTTP_AUTH_USER_ID.scope(user_id, next.run(request)).await,
                Err(_) => (StatusCode::UNAUTHORIZED, "Invalid API token").into_response(),
            };
        }

        return (StatusCode::UNAUTHORIZED, "Invalid Authorization header").into_response();
    }

    next.run(request).await
}
