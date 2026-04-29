use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tokio::sync::Mutex;
use tower_sessions::Session;

use crate::domain::user::User;
use crate::domain::user_repository::UserRepository;
use crate::infrastructure::auth::session;

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
    let user_id = state
        .user_command_service
        .resolve_api_token(token, &state.pool)
        .await
        .ok()?;

    state.user_repo.find_by_id(&user_id).await.ok()
}

/// Extract the current user from request extensions
pub fn get_current_user(request: &Request) -> Option<&User> {
    request.extensions().get::<User>()
}

/// Simple IP-based rate limiter using a sliding window
pub struct RateLimiter {
    max_requests: u32,
    window_secs: u64,
    requests: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            requests: Mutex::new(HashMap::new()),
        }
    }

    async fn check(&self, ip: IpAddr) -> bool {
        let mut map = self.requests.lock().await;
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        let timestamps = map.entry(ip).or_default();
        timestamps.retain(|t| now.duration_since(*t) < window);

        if timestamps.len() >= self.max_requests as usize {
            false
        } else {
            timestamps.push(now);
            true
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    // Extract client IP from X-Forwarded-For or connection info
    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip())
        })
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    if limiter.check(ip).await {
        next.run(request).await
    } else {
        (StatusCode::TOO_MANY_REQUESTS, "Too many requests. Please try again later.").into_response()
    }
}
