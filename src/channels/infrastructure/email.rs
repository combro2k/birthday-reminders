use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};

use crate::channels::domain::notification::{NotificationError, NotificationSender};
use crate::channels::domain::notification_config::{EmailConfig, SmtpSecurity};
use crate::reminders::domain::reminder::PendingReminder;

pub struct EmailSender {
    config: EmailConfig,
    unsubscribe_url: Option<String>,
    list_id_domain: Option<String>,
}

impl EmailSender {
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config,
            unsubscribe_url: None,
            list_id_domain: None,
        }
    }

    pub fn with_unsubscribe(mut self, url: String, domain: String) -> Self {
        self.unsubscribe_url = Some(url);
        self.list_id_domain = Some(domain);
        self
    }

    fn build_transport(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>, NotificationError> {
        let host = self.config.resolved_host();
        let port = self.config.resolved_port();
        let security = self.config.resolved_security();

        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());

        let transport = match security {
            SmtpSecurity::Starttls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?
                .port(port)
                .credentials(creds)
                .build(),
            SmtpSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                .map_err(|e| NotificationError::InvalidConfig(e.to_string()))?
                .port(port)
                .credentials(creds)
                .build(),
            SmtpSecurity::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .credentials(creds)
                .build(),
        };

        Ok(transport)
    }

    fn build_message(&self, subject: &str, body: &str) -> Result<Message, NotificationError> {
        let from = self.config.username.clone();
        let to = self.config.to.clone();

        let mut builder = Message::builder()
            .from(from.parse().map_err(|e| {
                NotificationError::InvalidConfig(format!("Invalid from address: {}", e))
            })?)
            .to(to.parse().map_err(|e| {
                NotificationError::InvalidConfig(format!("Invalid to address: {}", e))
            })?)
            .subject(subject);

        // RFC 2369 / RFC 8058: List-Unsubscribe headers
        if let Some(ref url) = self.unsubscribe_url {
            builder = builder
                .header(ListUnsubscribe(format!("<{}>", url)))
                .header(ListUnsubscribePost(
                    "List-Unsubscribe=One-Click".to_string(),
                ));
        }

        // RFC 2919: List-Id
        if let Some(ref domain) = self.list_id_domain {
            builder = builder.header(ListId(format!("<birthday-reminders.{}>", domain)));
        }

        builder
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| NotificationError::SendFailed(e.to_string()))
    }

    async fn send_email(&self, subject: &str, body: &str) -> Result<(), NotificationError> {
        let email = self.build_message(subject, body)?;

        let transport = self.build_transport()?;
        transport
            .send(email)
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        Ok(())
    }
}

/// RFC 2369 List-Unsubscribe header
#[derive(Clone)]
struct ListUnsubscribe(String);

impl lettre::message::header::Header for ListUnsubscribe {
    fn name() -> lettre::message::header::HeaderName {
        lettre::message::header::HeaderName::new_from_ascii_str("List-Unsubscribe")
    }

    fn parse(_: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self(String::new()))
    }

    fn display(&self) -> lettre::message::header::HeaderValue {
        lettre::message::header::HeaderValue::new(Self::name(), self.0.clone())
    }
}

/// RFC 8058 List-Unsubscribe-Post header
#[derive(Clone)]
struct ListUnsubscribePost(String);

impl lettre::message::header::Header for ListUnsubscribePost {
    fn name() -> lettre::message::header::HeaderName {
        lettre::message::header::HeaderName::new_from_ascii_str("List-Unsubscribe-Post")
    }

    fn parse(_: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self(String::new()))
    }

    fn display(&self) -> lettre::message::header::HeaderValue {
        lettre::message::header::HeaderValue::new(Self::name(), self.0.clone())
    }
}

/// RFC 2919 List-Id header
#[derive(Clone)]
struct ListId(String);

impl lettre::message::header::Header for ListId {
    fn name() -> lettre::message::header::HeaderName {
        lettre::message::header::HeaderName::new_from_ascii_str("List-Id")
    }

    fn parse(_: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self(String::new()))
    }

    fn display(&self) -> lettre::message::header::HeaderValue {
        lettre::message::header::HeaderValue::new(Self::name(), self.0.clone())
    }
}

#[async_trait]
impl NotificationSender for EmailSender {
    async fn send(&self, reminder: &PendingReminder) -> Result<(), NotificationError> {
        let subject = format!("Birthday Reminder: {}", reminder.birthday.name);
        let body = reminder.message();
        self.send_email(&subject, &body).await
    }

    async fn test(&self) -> Result<(), NotificationError> {
        self.send_email(
            "Birthday Reminders - Test",
            "This is a test email from Birthday Reminders. Your email configuration is working!",
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> EmailConfig {
        EmailConfig {
            provider: crate::channels::domain::notification_config::EmailProvider::Gmail,
            username: "sender@example.com".to_string(),
            password: "secret".to_string(),
            to: "to@example.com".to_string(),
            smtp_host: None,
            smtp_port: None,
            security: None,
        }
    }

    #[test]
    fn message_contains_list_headers_when_unsubscribe_configured() {
        let sender = EmailSender::new(base_config()).with_unsubscribe(
            "https://example.com/unsubscribe?token=us_abc".to_string(),
            "example.com".to_string(),
        );

        let msg = sender
            .build_message("Birthday Reminder", "Body")
            .expect("build message");
        let raw = String::from_utf8_lossy(&msg.formatted()).to_string();

        assert!(raw.contains("List-Unsubscribe: <https://example.com/unsubscribe?token=us_abc>"));
        assert!(raw.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click"));
        assert!(raw.contains("List-Id: <birthday-reminders.example.com>"));
    }

    #[test]
    fn message_omits_list_headers_when_unsubscribe_not_configured() {
        let sender = EmailSender::new(base_config());

        let msg = sender
            .build_message("Birthday Reminder", "Body")
            .expect("build message");
        let raw = String::from_utf8_lossy(&msg.formatted()).to_string();

        assert!(!raw.contains("List-Unsubscribe:"));
        assert!(!raw.contains("List-Unsubscribe-Post:"));
        assert!(!raw.contains("List-Id:"));
    }
}
