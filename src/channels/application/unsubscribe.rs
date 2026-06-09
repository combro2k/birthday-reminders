use std::sync::Arc;

use tracing::warn;

use crate::channels::domain::repository::NotificationChannelRepository;
use crate::channels::domain::unsubscribe_token::UnsubscribeToken;
use crate::channels::domain::unsubscribe_token_repository::UnsubscribeTokenRepository;
use crate::infrastructure::error::RepositoryError;
use crate::users::domain::user::UserId;

pub struct UnsubscribeService {
    token_repo: Arc<dyn UnsubscribeTokenRepository>,
    notification_repo: Arc<dyn NotificationChannelRepository>,
}

impl UnsubscribeService {
    pub fn new(
        token_repo: Arc<dyn UnsubscribeTokenRepository>,
        notification_repo: Arc<dyn NotificationChannelRepository>,
    ) -> Self {
        Self {
            token_repo,
            notification_repo,
        }
    }

    /// Get an existing active token or create a new one for the user + channel.
    /// Returns the full unsubscribe URL.
    pub async fn get_or_create_unsubscribe_url(
        &self,
        user_id: &UserId,
        channel_type: &str,
        base_url: &str,
    ) -> anyhow::Result<String> {
        // Check for existing active token
        if let Some(existing) = self
            .token_repo
            .find_active_for_user_channel(user_id, channel_type)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query token: {}", e))?
        {
            return Ok(format_url(base_url, &existing.token));
        }

        // Create a new token
        let token = UnsubscribeToken::new(user_id.clone(), channel_type.to_string());
        self.token_repo
            .create(&token)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create unsubscribe token: {}", e))?;

        Ok(format_url(base_url, &token.token))
    }

    /// Process an unsubscribe request. Validates the token and disables the channel.
    pub async fn process_unsubscribe(&self, token_value: &str) -> anyhow::Result<()> {
        let token = self
            .token_repo
            .find_by_token(token_value)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query token: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Invalid unsubscribe token"))?;

        // Check if already used — return success (idempotent)
        if token.used_at.is_some() {
            return Ok(());
        }

        // Check expiry
        if !token.is_valid() {
            anyhow::bail!("Unsubscribe token has expired");
        }

        // Disable the notification channel
        let channel = self
            .notification_repo
            .find_by_type(&token.user_id, &token.channel_type)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query channel: {}", e))?;

        if let Some(ch) = channel
            && ch.enabled
        {
            self.notification_repo
                .upsert(
                    &token.user_id,
                    &token.channel_type,
                    false,
                    ch.config.clone(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to disable channel: {}", e))?;
        }

        // Mark token as used
        if let Err(e) = self.token_repo.mark_used(token_value).await {
            warn!("Failed to mark unsubscribe token as used: {}", e);
        }

        Ok(())
    }

    /// Find token by value (for rendering the landing page).
    pub async fn find_token(
        &self,
        token_value: &str,
    ) -> Result<Option<UnsubscribeToken>, RepositoryError> {
        self.token_repo.find_by_token(token_value).await
    }

    /// Invalidate all tokens for a user + channel (call when channel is re-enabled).
    pub async fn invalidate_tokens(
        &self,
        user_id: &UserId,
        channel_type: &str,
    ) -> anyhow::Result<()> {
        self.token_repo
            .delete_for_user_channel(user_id, channel_type)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to invalidate tokens: {}", e))
    }
}

fn format_url(base_url: &str, token: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{}/unsubscribe?token={}", base, token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_url() {
        assert_eq!(
            format_url("https://example.com", "us_abc123"),
            "https://example.com/unsubscribe?token=us_abc123"
        );
        assert_eq!(
            format_url("https://example.com/", "us_abc123"),
            "https://example.com/unsubscribe?token=us_abc123"
        );
    }
}
