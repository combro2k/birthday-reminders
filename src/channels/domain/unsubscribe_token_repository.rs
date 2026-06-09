use async_trait::async_trait;

use crate::infrastructure::error::RepositoryError;
use crate::users::domain::user::UserId;

use super::unsubscribe_token::UnsubscribeToken;

#[async_trait]
pub trait UnsubscribeTokenRepository: Send + Sync {
    /// Find a token by its string value.
    async fn find_by_token(&self, token: &str)
    -> Result<Option<UnsubscribeToken>, RepositoryError>;

    /// Find the active (unused, non-expired) token for a user + channel.
    async fn find_active_for_user_channel(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<Option<UnsubscribeToken>, RepositoryError>;

    /// Persist a new token.
    async fn create(&self, token: &UnsubscribeToken) -> Result<(), RepositoryError>;

    /// Mark a token as used (sets `used_at` to now).
    async fn mark_used(&self, token: &str) -> Result<(), RepositoryError>;

    /// Delete all tokens for a user + channel (used when channel is re-enabled).
    async fn delete_for_user_channel(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> Result<(), RepositoryError>;
}
