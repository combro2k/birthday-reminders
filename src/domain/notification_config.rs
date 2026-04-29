use serde::{Deserialize, Serialize};

/// Gotify channel configuration (user-provided)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotifyConfig {
    pub url: String,
    pub token: String,
}

/// Email channel configuration (user-provided)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub provider: EmailProvider,
    pub username: String,
    pub password: String,
    pub to: String,
    /// Only used when provider is Custom
    pub smtp_host: Option<String>,
    /// Only used when provider is Custom
    pub smtp_port: Option<u16>,
    /// Only used when provider is Custom
    pub security: Option<SmtpSecurity>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmailProvider {
    Gmail,
    Proton,
    Outlook,
    Custom,
}

impl EmailProvider {
    pub fn smtp_host(&self) -> Option<&'static str> {
        match self {
            EmailProvider::Gmail => Some("smtp.gmail.com"),
            EmailProvider::Proton => Some("127.0.0.1"),
            EmailProvider::Outlook => Some("smtp.office365.com"),
            EmailProvider::Custom => None,
        }
    }

    pub fn smtp_port(&self) -> Option<u16> {
        match self {
            EmailProvider::Gmail => Some(587),
            EmailProvider::Proton => Some(1025),
            EmailProvider::Outlook => Some(587),
            EmailProvider::Custom => None,
        }
    }

    pub fn security(&self) -> SmtpSecurity {
        match self {
            EmailProvider::Gmail => SmtpSecurity::Starttls,
            EmailProvider::Proton => SmtpSecurity::Starttls,
            EmailProvider::Outlook => SmtpSecurity::Starttls,
            EmailProvider::Custom => SmtpSecurity::Starttls,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SmtpSecurity {
    Starttls,
    Tls,
    None,
}

impl EmailConfig {
    pub fn resolved_host(&self) -> &str {
        if self.provider == EmailProvider::Custom {
            self.smtp_host.as_deref().unwrap_or("localhost")
        } else {
            self.provider.smtp_host().unwrap_or("localhost")
        }
    }

    pub fn resolved_port(&self) -> u16 {
        if self.provider == EmailProvider::Custom {
            self.smtp_port.unwrap_or(587)
        } else {
            self.provider.smtp_port().unwrap_or(587)
        }
    }

    pub fn resolved_security(&self) -> SmtpSecurity {
        if self.provider == EmailProvider::Custom {
            self.security.unwrap_or(SmtpSecurity::Starttls)
        } else {
            self.provider.security()
        }
    }
}

/// Telegram channel configuration (user-provided)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

/// Signal channel configuration (user-provided, stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    pub api_url: String,
    pub recipient: String,
}

/// WhatsApp channel configuration (user-provided, stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsappConfig {
    pub api_url: String,
    pub recipient: String,
}
