use async_trait::async_trait;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

use crate::domain::notification::{ChannelKind, NotificationError, NotificationSender};
use crate::domain::notification_config::{EmailConfig, SmtpSecurity};
use crate::domain::reminder::PendingReminder;

pub struct EmailSender {
    config: EmailConfig,
}

impl EmailSender {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    fn build_transport(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>, NotificationError> {
        let host = self.config.resolved_host();
        let port = self.config.resolved_port();
        let security = self.config.resolved_security();

        let creds = Credentials::new(
            self.config.username.clone(),
            self.config.password.clone(),
        );

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

    async fn send_email(&self, subject: &str, body: &str) -> Result<(), NotificationError> {
        let from = self.config.username.clone();
        let to = self.config.to.clone();

        let email = Message::builder()
            .from(from.parse().map_err(|e| {
                NotificationError::InvalidConfig(format!("Invalid from address: {}", e))
            })?)
            .to(to.parse().map_err(|e| {
                NotificationError::InvalidConfig(format!("Invalid to address: {}", e))
            })?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        let transport = self.build_transport()?;
        transport
            .send(email)
            .await
            .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl NotificationSender for EmailSender {
    fn channel_kind(&self) -> ChannelKind {
        ChannelKind::Email
    }

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
