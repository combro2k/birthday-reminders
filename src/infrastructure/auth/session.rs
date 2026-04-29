use tower_sessions::Session;
use uuid::Uuid;

use crate::domain::user::UserId;

const USER_ID_KEY: &str = "user_id";

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
