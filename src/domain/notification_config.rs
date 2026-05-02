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
    #[serde(rename = "proton_smtp")]
    ProtonSmtp,
    Outlook,
    Custom,
}

impl EmailProvider {
    pub fn smtp_host(&self) -> Option<&'static str> {
        match self {
            EmailProvider::Gmail => Some("smtp.gmail.com"),
            EmailProvider::Proton => Some("127.0.0.1"),
            EmailProvider::ProtonSmtp => Some("smtp.protonmail.ch"),
            EmailProvider::Outlook => Some("smtp.office365.com"),
            EmailProvider::Custom => None,
        }
    }

    pub fn smtp_port(&self) -> Option<u16> {
        match self {
            EmailProvider::Gmail => Some(587),
            EmailProvider::Proton => Some(1025),
            EmailProvider::ProtonSmtp => Some(587),
            EmailProvider::Outlook => Some(587),
            EmailProvider::Custom => None,
        }
    }

    pub fn security(&self) -> SmtpSecurity {
        match self {
            EmailProvider::Gmail => SmtpSecurity::Starttls,
            EmailProvider::Proton => SmtpSecurity::Starttls,
            EmailProvider::ProtonSmtp => SmtpSecurity::Starttls,
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

/// Discord channel configuration (user-provided, stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

#[cfg(test)]
mod tests {
    use super::{EmailConfig, EmailProvider, SmtpSecurity};

    #[test]
    fn proton_bridge_defaults_remain_localhost() {
        assert_eq!(EmailProvider::Proton.smtp_host(), Some("127.0.0.1"));
        assert_eq!(EmailProvider::Proton.smtp_port(), Some(1025));
        assert_eq!(EmailProvider::Proton.security(), SmtpSecurity::Starttls);
    }

    #[test]
    fn proton_smtp_submission_defaults_are_resolved() {
        let config = EmailConfig {
            provider: EmailProvider::ProtonSmtp,
            username: "you@proton.me".to_string(),
            password: "smtp-token".to_string(),
            to: "you@proton.me".to_string(),
            smtp_host: None,
            smtp_port: None,
            security: None,
        };

        assert_eq!(config.resolved_host(), "smtp.protonmail.ch");
        assert_eq!(config.resolved_port(), 587);
        assert_eq!(config.resolved_security(), SmtpSecurity::Starttls);
    }
}
