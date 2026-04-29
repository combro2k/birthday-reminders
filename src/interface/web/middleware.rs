use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{Method, StatusCode},
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

/// CSRF protection middleware for POST requests.
/// Validates that the `csrf_token` form field or header matches the session token.
pub async fn csrf_middleware(
    session: Session,
    request: Request,
    next: Next,
) -> Response {
    // Only validate POST/PUT/PATCH/DELETE requests
    if request.method() != Method::POST
        && request.method() != Method::PUT
        && request.method() != Method::PATCH
        && request.method() != Method::DELETE
    {
        return next.run(request).await;
    }

    // Skip CSRF check for API requests (they use Bearer tokens)
    if request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("Bearer "))
    {
        return next.run(request).await;
    }

    // Extract CSRF token from the form body
    // We need to buffer the body to read the form data and then reconstruct it
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    // Try to find csrf_token in URL-encoded form data
    let body_str = String::from_utf8_lossy(&bytes);
    let csrf_token = form_field_value(&body_str, "csrf_token");

    // Also check header as fallback
    let csrf_token = csrf_token.or_else(|| {
        parts
            .headers
            .get("x-csrf-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    });

    let valid = match csrf_token {
        Some(token) => {
            crate::infrastructure::auth::session::validate_csrf_token(&session, &token).await
        }
        None => false,
    };

    if !valid {
        return (StatusCode::FORBIDDEN, "Invalid or missing CSRF token").into_response();
    }

    // Reconstruct the request with the buffered body, stripping the csrf_token field
    let filtered_body: String = url::form_urlencoded::parse(&bytes)
        .filter(|(key, _)| key != "csrf_token")
        .map(|(k, v)| format!("{}={}", url::form_urlencoded::byte_serialize(k.as_bytes()).collect::<String>(), url::form_urlencoded::byte_serialize(v.as_bytes()).collect::<String>()))
        .collect::<Vec<_>>()
        .join("&");
    let request = Request::from_parts(parts, Body::from(filtered_body));
    next.run(request).await
}

/// Extract a form field value from URL-encoded form data
fn form_field_value(body: &str, field: &str) -> Option<String> {
    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect();
    pairs.into_iter().find(|(k, _)| k == field).map(|(_, v)| v)
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
