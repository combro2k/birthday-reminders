//! WhatsApp Cloud API notification sender.
//!
//! Sends birthday reminders via Meta's WhatsApp Cloud API with automatic retry and backoff for transient failures.
//! Implements the [NotificationSender] trait and integrates with the reminder dispatch pipeline.
//!
//! # Configuration
//! WhatsAppSender requires:
//! - `phone_number_id`: The WhatsApp Business Account phone number ID from Meta Business Manager
//! - `access_token`: A permanent access token with `whatsapp_business_messaging` permission
//! - `recipient_phone`: The recipient's phone number in E.164 format (without +, e.g. "15551234567")
//!
//! # Retry Behavior
//! Transient failures (429 rate limit, 5xx server errors, timeouts, and connection failures)
//! are retried up to 4 times with exponential backoff (500ms → 1s → 2s → 4s → 8s max).
//! Non-retryable errors (4xx client errors except 429) fail immediately.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use tokio::time::{Duration, sleep};

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::WhatsappConfig;
use crate::reminders::domain::reminder::PendingReminder;

/// Sends messages via the Meta WhatsApp Cloud API.
pub struct WhatsappSender {
    config: WhatsappConfig,
    client: Client,
}

impl WhatsappSender {
    /// Create a new WhatsApp sender from configuration.
    pub fn new(config: WhatsappConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Build the Meta Graph API endpoint URL for this phone number ID.
    /// Uses the v22.0 API endpoint to send text messages.
    fn endpoint_url(&self) -> String {
        format!(
            "https://graph.facebook.com/v22.0/{}/messages",
            self.config.phone_number_id
        )
    }

    /// Determine if a given HTTP status code warrants a retry.
    /// Returns true for rate limit (429) and server errors (5xx).
    fn should_retry_status(status: StatusCode) -> bool {
        status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
    }

    /// Send a text message via WhatsApp Cloud API with retry and exponential backoff.
    /// Retries transient failures up to 4 times with backoff capped at 8 seconds.
    /// Returns Ok(()) on success; NotificationError::SendFailed on failure after all retries.
    async fn send_message(&self, text: &str) -> Result<(), NotificationError> {
        let url = self.endpoint_url();
        // Construct JSON payload matching Meta's Cloud API text message schema
        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": self.config.recipient_phone,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": text,
            }
        });

        let max_attempts = 4;
        let mut backoff = Duration::from_millis(500);

        for attempt in 1..=max_attempts {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.config.access_token)
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        return Ok(());
                    }

                    let status = resp.status();
                    let response_body = resp.text().await.unwrap_or_default();

                    if attempt < max_attempts && Self::should_retry_status(status) {
                        sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(8));
                        continue;
                    }

                    return Err(NotificationError::SendFailed(format!(
                        "WhatsApp Cloud API returned {}: {}",
                        status, response_body
                    )));
                }
                Err(err) => {
                    let transient = err.is_timeout() || err.is_connect() || err.is_request();
                    if attempt < max_attempts && transient {
                        sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(8));
                        continue;
                    }

                    return Err(NotificationError::SendFailed(format!(
                        "WhatsApp request failed: {}",
                        err
                    )));
                }
            }
        }

        Err(NotificationError::SendFailed(
            "WhatsApp delivery failed after retries".to_string(),
        ))
    }
}

#[async_trait]
impl NotificationSender for WhatsappSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        let text = format!("{}\n\n{}", reminder.title(), reminder.message());
        self.send_message(&text).await
    }

    async fn test(&self) -> Result<(), NotificationError> {
        self.send_message(
            "Birthday Reminders test notification. Your WhatsApp channel configuration is working.",
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::WhatsappSender;
    use reqwest::StatusCode;

    #[test]
    fn retries_for_rate_limit_or_server_errors() {
        assert!(WhatsappSender::should_retry_status(
            StatusCode::TOO_MANY_REQUESTS
        ));
        assert!(WhatsappSender::should_retry_status(
            StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(WhatsappSender::should_retry_status(StatusCode::BAD_GATEWAY));
        assert!(!WhatsappSender::should_retry_status(
            StatusCode::BAD_REQUEST
        ));
        assert!(!WhatsappSender::should_retry_status(
            StatusCode::UNAUTHORIZED
        ));
    }
}
