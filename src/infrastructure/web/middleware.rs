use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::HeaderMap,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use ipnet::IpNet;
use tokio::sync::Mutex;
use tower_sessions::Session;

use crate::auth::infrastructure::session;
use crate::users::domain::user::User;

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
    if let Some(auth_header) = request.headers().get("authorization")
        && let Ok(header_str) = auth_header.to_str()
        && let Some(token) = header_str.strip_prefix("Bearer ")
        && let Some(user) = validate_bearer_token(token, &state).await
    {
        request.extensions_mut().insert(user);
        return next.run(request).await;
    }

    // Log the authentication failure with client IP
    let client_ip = request
        .extensions()
        .get::<ClientInfo>()
        .map(|ci| ci.ip);
    let path = request.uri().path().to_string();

    // Check if this is an API request or browser request
    let is_api = request
        .headers()
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/json"));

    if is_api {
        tracing::warn!(
            client_ip = ?client_ip,
            path = %path,
            method = "bearer",
            "authentication failed for API request"
        );
        StatusCode::UNAUTHORIZED.into_response()
    } else {
        tracing::warn!(
            client_ip = ?client_ip,
            path = %path,
            "unauthenticated browser request, redirecting to login"
        );
        Redirect::to("/auth/login").into_response()
    }
}

async fn validate_bearer_token(token: &str, state: &AppState) -> Option<User> {
    let user_id = state
        .user_command_service
        .resolve_api_token(token, &state.db)
        .await
        .ok()?;

    state.user_repo.find_by_id(&user_id).await.ok()
}

/// CSRF protection middleware for POST requests.
/// Validates that the `csrf_token` form field or header matches the session token.
pub async fn csrf_middleware(session: Session, request: Request, next: Next) -> Response {
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
            crate::auth::infrastructure::session::validate_csrf_token(&session, &token).await
        }
        None => false,
    };

    if !valid {
        return (StatusCode::FORBIDDEN, "Invalid or missing CSRF token").into_response();
    }

    // Reconstruct the request with the buffered body, stripping the csrf_token field
    let filtered_body: String = url::form_urlencoded::parse(&bytes)
        .filter(|(key, _)| key != "csrf_token")
        .map(|(k, v)| {
            format!(
                "{}={}",
                url::form_urlencoded::byte_serialize(k.as_bytes()).collect::<String>(),
                url::form_urlencoded::byte_serialize(v.as_bytes()).collect::<String>()
            )
        })
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

/// Resolved client connection info after processing reverse proxy headers.
/// Inserted into request extensions by [`proxy_headers_middleware`].
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub ip: IpAddr,
    pub scheme: Option<String>,
    pub host: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProxyTrust {
    trusted_proxies: Vec<IpNet>,
}

impl ProxyTrust {
    pub fn new(trusted_proxies: Vec<IpNet>) -> Self {
        Self { trusted_proxies }
    }

    /// Resolve all client information from proxy headers when the peer is trusted.
    pub fn client_info(&self, headers: &HeaderMap, peer_ip: Option<IpAddr>) -> ClientInfo {
        let ip = self
            .client_ip(headers, peer_ip)
            .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

        let trusted = peer_ip.is_some_and(|p| self.is_trusted(p));

        let scheme = if trusted {
            self.forwarded_scheme(headers)
        } else {
            None
        };

        let host = if trusted {
            self.forwarded_host(headers)
        } else {
            None
        };

        ClientInfo { ip, scheme, host }
    }

    pub fn client_ip(&self, headers: &HeaderMap, peer_ip: Option<IpAddr>) -> Option<IpAddr> {
        let peer_ip = peer_ip?;
        if !self.is_trusted(peer_ip) {
            return Some(peer_ip);
        }

        self.client_ip_from_forwarded_headers(headers)
            .or(Some(peer_ip))
    }

    fn client_ip_from_forwarded_headers(&self, headers: &HeaderMap) -> Option<IpAddr> {
        if let Some(chain) = parse_forwarded_for(headers) {
            for address in chain.iter().rev() {
                if !self.is_trusted(*address) {
                    return Some(*address);
                }
            }

            return chain.first().copied();
        }

        headers.get("x-real-ip").and_then(parse_single_ip_header)
    }

    fn forwarded_scheme(&self, headers: &HeaderMap) -> Option<String> {
        let value = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_ascii_lowercase())?;

        // Only accept valid schemes
        if matches!(value.as_str(), "http" | "https") {
            Some(value)
        } else {
            None
        }
    }

    fn forwarded_host(&self, headers: &HeaderMap) -> Option<String> {
        let value = headers
            .get("x-forwarded-host")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_string())?;

        // Reject empty or obviously invalid host values
        if value.is_empty() || value.contains('/') || value.contains('\\') {
            return None;
        }

        Some(value)
    }

    fn is_trusted(&self, ip: IpAddr) -> bool {
        self.trusted_proxies
            .iter()
            .any(|network| network.contains(&ip))
    }
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

/// Middleware that resolves reverse proxy headers into [`ClientInfo`] and inserts it
/// into request extensions. Must be applied as an outer layer so inner handlers and
/// middleware can access the resolved client information.
pub async fn proxy_headers_middleware(
    proxy_trust: Arc<ProxyTrust>,
    request: Request,
    next: Next,
) -> Response {
    let peer_ip = request
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip());
    let client_info = proxy_trust.client_info(request.headers(), peer_ip);

    tracing::debug!(
        client_ip = %client_info.ip,
        forwarded_proto = client_info.scheme.as_deref().unwrap_or("-"),
        forwarded_host = client_info.host.as_deref().unwrap_or("-"),
        "resolved client info from proxy headers"
    );

    let mut request = request;
    request.extensions_mut().insert(client_info);
    next.run(request).await
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let ip = request
        .extensions()
        .get::<ClientInfo>()
        .map(|ci| ci.ip)
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    if limiter.check(ip).await {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many requests. Please try again later.",
        )
            .into_response()
    }
}

fn parse_forwarded_for(headers: &HeaderMap) -> Option<Vec<IpAddr>> {
    let value = headers.get("x-forwarded-for")?.to_str().ok()?;
    let addresses: Option<Vec<IpAddr>> = value
        .split(',')
        .map(|segment| segment.trim().parse::<IpAddr>().ok())
        .collect();

    addresses.filter(|addresses| !addresses.is_empty())
}

fn parse_single_ip_header(value: &axum::http::HeaderValue) -> Option<IpAddr> {
    value.to_str().ok()?.trim().parse::<IpAddr>().ok()
}

#[cfg(test)]
mod tests {
    use super::ProxyTrust;
    use axum::http::HeaderMap;
    use ipnet::IpNet;
    use std::net::{IpAddr, Ipv4Addr};

    fn proxy_trust() -> ProxyTrust {
        ProxyTrust::new(vec![
            "127.0.0.1/32".parse::<IpNet>().unwrap(),
            "10.0.0.0/8".parse::<IpNet>().unwrap(),
        ])
    }

    #[test]
    fn uses_peer_ip_when_peer_is_not_trusted() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.10".parse().unwrap());

        let ip = trust.client_ip(&headers, Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10))));

        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10))));
    }

    #[test]
    fn uses_first_untrusted_ip_from_right_side_of_forwarded_chain() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "203.0.113.10, 10.0.0.5, 127.0.0.1".parse().unwrap(),
        );

        let ip = trust.client_ip(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))));
    }

    #[test]
    fn falls_back_to_x_real_ip_for_trusted_proxy() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.44".parse().unwrap());

        let ip = trust.client_ip(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 44))));
    }

    #[test]
    fn ignores_malformed_forwarded_for_chain() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "invalid, 203.0.113.10".parse().unwrap());

        let ip = trust.client_ip(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    }

    // --- X-Forwarded-Proto tests ---

    #[test]
    fn extracts_forwarded_proto_from_trusted_proxy() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "https".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(info.scheme.as_deref(), Some("https"));
    }

    #[test]
    fn ignores_forwarded_proto_from_untrusted_peer() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "https".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1))));

        assert_eq!(info.scheme, None);
    }

    #[test]
    fn rejects_invalid_forwarded_proto() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "ftp".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(info.scheme, None);
    }

    #[test]
    fn normalises_forwarded_proto_to_lowercase() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "HTTPS".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(info.scheme.as_deref(), Some("https"));
    }

    // --- X-Forwarded-Host tests ---

    #[test]
    fn extracts_forwarded_host_from_trusted_proxy() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-host", "birthdays.example.com".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(info.host.as_deref(), Some("birthdays.example.com"));
    }

    #[test]
    fn extracts_forwarded_host_with_port() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-host",
            "birthdays.example.com:8443".parse().unwrap(),
        );

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(info.host.as_deref(), Some("birthdays.example.com:8443"));
    }

    #[test]
    fn ignores_forwarded_host_from_untrusted_peer() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-host", "evil.example.com".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1))));

        assert_eq!(info.host, None);
    }

    #[test]
    fn rejects_forwarded_host_with_path() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-host", "evil.com/path".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(info.host, None);
    }

    // --- ClientInfo integration tests ---

    #[test]
    fn client_info_resolves_all_fields_from_trusted_proxy() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.50".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("x-forwarded-host", "birthdays.example.com".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));

        assert_eq!(info.ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 50)));
        assert_eq!(info.scheme.as_deref(), Some("https"));
        assert_eq!(info.host.as_deref(), Some("birthdays.example.com"));
    }

    #[test]
    fn client_info_ignores_all_forwarded_headers_from_untrusted_peer() {
        let trust = proxy_trust();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.50".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("x-forwarded-host", "evil.example.com".parse().unwrap());

        let info = trust.client_info(&headers, Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1))));

        // Untrusted peer: use the peer IP directly, ignore forwarded headers
        assert_eq!(info.ip, IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)));
        assert_eq!(info.scheme, None);
        assert_eq!(info.host, None);
    }
}
