use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::infrastructure::web::server::AppState;
use crate::mcp::application::birthdays::HTTP_AUTH_USER_ID;
use crate::mcp::infrastructure::session::AuthenticatedSessionManager;

/// Shared state passed to the MCP auth middleware.
#[derive(Clone)]
pub struct McpAuthState {
    pub app: Arc<AppState>,
    pub sessions: Arc<AuthenticatedSessionManager>,
}

impl McpAuthState {
    pub fn new(app: Arc<AppState>, sessions: Arc<AuthenticatedSessionManager>) -> Self {
        Self { app, sessions }
    }
}

const MCP_SESSION_ID_HEADER: &str = "mcp-session-id";

pub async fn mcp_auth_middleware(
    State(mcp): State<McpAuthState>,
    request: Request,
    next: Next,
) -> Response {
    let session_id = request
        .headers()
        .get(MCP_SESSION_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned);

    let bearer_token = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned);

    // 1. Try session-based auth: reuse a previously authenticated session.
    if let Some(ref sid) = session_id
        && let Some(user_id) = mcp.sessions.get_user(sid).await
    {
        return HTTP_AUTH_USER_ID.scope(user_id, next.run(request)).await;
    }

    // 2. Try Bearer token auth.
    if let Some(ref token) = bearer_token {
        return match mcp
            .app
            .user_command_service
            .resolve_api_token(token, &mcp.app.db)
            .await
        {
            Ok(user_id) => {
                let response = HTTP_AUTH_USER_ID
                    .scope(user_id.clone(), next.run(request))
                    .await;

                // Bind the authenticated user to the MCP session so
                // subsequent requests skip token validation.
                let sid = session_id.or_else(|| {
                    response
                        .headers()
                        .get(MCP_SESSION_ID_HEADER)
                        .and_then(|h| h.to_str().ok())
                        .map(str::to_owned)
                });
                if let Some(sid) = sid {
                    mcp.sessions.bind_user(&sid, user_id).await;
                }

                response
            }
            Err(_) => (StatusCode::UNAUTHORIZED, "Invalid API token").into_response(),
        };
    }

    // 3. Check for a malformed Authorization header (present but not Bearer).
    if request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .is_some_and(|v| !v.starts_with("Bearer "))
    {
        return (StatusCode::UNAUTHORIZED, "Invalid Authorization header").into_response();
    }

    // 4. No auth — pass through. Public tools (e.g. setup guide) will work;
    //    protected tools will fail in resolve_user_id().
    next.run(request).await
}
