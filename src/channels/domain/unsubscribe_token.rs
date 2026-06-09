use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::users::domain::user::UserId;

/// An unsubscribe token allows one-click email unsubscription (RFC 8058).
#[derive(Debug, Clone)]
pub struct UnsubscribeToken {
    pub id: Uuid,
    pub user_id: UserId,
    pub channel_type: String,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub used_at: Option<DateTime<Utc>>,
}

impl UnsubscribeToken {
    /// Create a new unsubscribe token for a user + channel combination.
    pub fn new(user_id: UserId, channel_type: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            channel_type,
            token: generate_token(),
            created_at: Utc::now(),
            expires_at: None,
            used_at: None,
        }
    }

    /// Returns true if the token can still be used for unsubscription.
    pub fn is_valid(&self) -> bool {
        if self.used_at.is_some() {
            return false;
        }
        if let Some(expires) = self.expires_at {
            return Utc::now() < expires;
        }
        true
    }
}

/// Generate a prefixed random token: `us_` + 32 hex chars (16 bytes).
fn generate_token() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    format!("us_{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_token_is_valid() {
        let token = UnsubscribeToken::new(UserId(Uuid::new_v4()), "email".to_string());
        assert!(token.is_valid());
        assert!(token.token.starts_with("us_"));
        assert_eq!(token.token.len(), 3 + 32); // "us_" + 32 hex chars
    }

    #[test]
    fn test_used_token_is_invalid() {
        let mut token = UnsubscribeToken::new(UserId(Uuid::new_v4()), "email".to_string());
        token.used_at = Some(Utc::now());
        assert!(!token.is_valid());
    }

    #[test]
    fn test_expired_token_is_invalid() {
        let mut token = UnsubscribeToken::new(UserId(Uuid::new_v4()), "email".to_string());
        token.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(!token.is_valid());
    }
}
