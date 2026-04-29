use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub reminders: RemindersConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub listen: String,
    pub base_url: String,
    pub session_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub allow_registration: bool,
    pub oidc: Option<OidcConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    #[serde(default)]
    pub enabled: bool,
    pub provider_name: String,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    #[serde(default = "default_true")]
    pub auto_provision: bool,
    #[serde(default = "default_role")]
    pub default_role: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemindersConfig {
    #[serde(default = "default_schedule")]
    pub schedule: String,
    #[serde(default = "default_days_before")]
    pub default_days_before: Vec<u32>,
}

fn default_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "profile".to_string(),
        "email".to_string(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_role() -> String {
    "user".to_string()
}

fn default_max_connections() -> u32 {
    10
}

fn default_schedule() -> String {
    "0 0 8 * * *".to_string()
}

fn default_days_before() -> Vec<u32> {
    vec![7, 3, 1, 0]
}

impl AppConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        // Session secret must be at least 32 bytes and not the example value
        if self.server.session_secret.len() < 32 {
            anyhow::bail!(
                "server.session_secret must be at least 32 characters (got {})",
                self.server.session_secret.len()
            );
        }

        const EXAMPLE_SECRET: &str =
            "change-me-to-a-random-64-character-string-in-production-please!";
        if self.server.session_secret == EXAMPLE_SECRET {
            anyhow::bail!(
                "server.session_secret is still set to the example value. \
                 Please generate a random secret for production use."
            );
        }

        Ok(())
    }
}
