use tower_sessions::Session;
use uuid::Uuid;

use crate::domain::user::UserId;

const USER_ID_KEY: &str = "user_id";
const CSRF_TOKEN_KEY: &str = "csrf_token";

pub async fn set_user_id(session: &Session, user_id: &UserId) -> anyhow::Result<()> {
    session
        .insert(USER_ID_KEY, user_id.0.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to set session: {}", e))?;
    Ok(())
}

pub async fn get_user_id(session: &Session) -> Option<UserId> {
    let value: Option<String> = session.get(USER_ID_KEY).await.ok().flatten();
    value
        .and_then(|s| Uuid::parse_str(&s).ok())
        .map(UserId)
}

pub async fn clear_session(session: &Session) {
    session.flush().await.ok();
}

/// Get or generate a CSRF token for this session
pub async fn get_csrf_token(session: &Session) -> String {
    if let Some(token) = session.get::<String>(CSRF_TOKEN_KEY).await.ok().flatten() {
        return token;
    }
    let token = Uuid::new_v4().to_string();
    let _ = session.insert(CSRF_TOKEN_KEY, &token).await;
    token
}

/// Validate a CSRF token against the one stored in the session
pub async fn validate_csrf_token(session: &Session, token: &str) -> bool {
    session
        .get::<String>(CSRF_TOKEN_KEY)
        .await
        .ok()
        .flatten()
        .is_some_and(|stored| stored == token)
}
