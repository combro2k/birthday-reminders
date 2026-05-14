use anyhow::Context;
use ipnet::IpNet;
use serde::Deserialize;
use std::path::Path;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub reminders: RemindersConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub commands: CommandsConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
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
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub server_name: Option<String>,
    #[serde(default = "default_scheme")]
    pub scheme: String,
    pub session_secret: String,
    /// Encryption key for notification channel secrets (required).
    pub encryption_key: String,
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
    /// User to drop privileges to after startup (required)
    pub run_as_user: String,
    /// Group to drop privileges to after startup (required)
    pub run_as_group: String,
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
    #[serde(default)]
    pub trusted_audiences: Vec<String>,
    #[serde(default)]
    pub allow_dynamic_additional_audiences: bool,
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

#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_mcp_path")]
    pub path: String,
    #[serde(default = "default_true")]
    pub stateful_mode: bool,
    #[serde(default)]
    pub json_response: bool,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: default_false(),
            path: default_mcp_path(),
            stateful_mode: default_true(),
            json_response: false,
            allowed_hosts: vec![],
            allowed_origins: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandsConfig {
    #[serde(default)]
    pub signal_transport: SignalTransportMode,
    #[serde(default = "default_signal_cli_path")]
    pub signal_cli_path: String,
    #[serde(default = "default_signal_api_url")]
    pub signal_api_url: String,
}

impl Default for CommandsConfig {
    fn default() -> Self {
        Self {
            signal_transport: SignalTransportMode::default(),
            signal_cli_path: default_signal_cli_path(),
            signal_api_url: default_signal_api_url(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SignalTransportMode {
    #[default]
    Cli,
    Api,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoggingConfig {
    /// Log output: "stdout" (default) or "syslog"
    #[serde(default = "default_log_output")]
    pub output: String,
    /// Log level filter (default: "info"). Supports RUST_LOG syntax.
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_output() -> String {
    "stdout".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_mcp_path() -> String {
    "/mcp".to_string()
}

fn default_false() -> bool {
    false
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

fn default_scheme() -> String {
    "http".to_string()
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

fn default_signal_cli_path() -> String {
    "signal-cli".to_string()
}

fn default_signal_api_url() -> String {
    "http://127.0.0.1:8080".to_string()
}

impl ServerConfig {
    pub fn public_base_url(&self) -> anyhow::Result<Url> {
        if let Some(base_url) = self.base_url.as_deref().map(str::trim)
            && !base_url.is_empty()
        {
            let url = Url::parse(base_url)
                .with_context(|| format!("Invalid server.base_url: {base_url}"))?;
            validate_public_url(&url)?;
            return Ok(url);
        }

        let server_name = self
            .server_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!("server.server_name must be set when server.base_url is omitted")
            })?;

        let scheme = self.scheme.trim().to_ascii_lowercase();
        validate_scheme(&scheme)?;

        let url = Url::parse(&format!("{scheme}://{server_name}"))
            .with_context(|| format!("Invalid server.server_name: {server_name}"))?;
        validate_public_url(&url)?;
        Ok(url)
    }

    pub fn secure_cookies(&self) -> anyhow::Result<bool> {
        Ok(self.public_base_url()?.scheme() == "https")
    }

    pub fn oidc_callback_url(&self) -> anyhow::Result<Url> {
        let mut base_url = self.public_base_url()?;
        let normalized_path = if base_url.path().is_empty() || base_url.path() == "/" {
            "/".to_string()
        } else if base_url.path().ends_with('/') {
            base_url.path().to_string()
        } else {
            format!("{}/", base_url.path())
        };
        base_url.set_path(&normalized_path);

        base_url
            .join("auth/oidc/callback")
            .context("Failed to construct OIDC callback URL")
    }

    pub fn trusted_proxy_nets(&self) -> anyhow::Result<Vec<IpNet>> {
        self.trusted_proxies
            .iter()
            .map(|entry| parse_trusted_proxy(entry))
            .collect()
    }
}

fn validate_scheme(scheme: &str) -> anyhow::Result<()> {
    if matches!(scheme, "http" | "https") {
        Ok(())
    } else {
        anyhow::bail!("server.scheme must be either 'http' or 'https'")
    }
}

fn validate_public_url(url: &Url) -> anyhow::Result<()> {
    validate_scheme(url.scheme())?;

    if url.host_str().is_none() {
        anyhow::bail!("server public URL must include a host")
    }
    if url.query().is_some() || url.fragment().is_some() {
        anyhow::bail!("server public URL must not include a query string or fragment")
    }

    Ok(())
}

fn parse_trusted_proxy(entry: &str) -> anyhow::Result<IpNet> {
    let entry = entry.trim();
    if entry.is_empty() {
        anyhow::bail!("server.trusted_proxies entries must not be empty")
    }

    if let Ok(network) = entry.parse::<IpNet>() {
        return Ok(network);
    }

    let address = entry
        .parse::<std::net::IpAddr>()
        .with_context(|| format!("Invalid trusted proxy IP or CIDR: {entry}"))?;
    Ok(IpNet::from(address))
}

impl AppConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        let _ = self.server.public_base_url()?;
        let _ = self.server.trusted_proxy_nets()?;

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

        // run_as_user and run_as_group must not be empty
        if self.server.run_as_user.trim().is_empty() {
            anyhow::bail!("server.run_as_user must be set (can be 'root' if desired)");
        }
        if self.server.run_as_group.trim().is_empty() {
            anyhow::bail!("server.run_as_group must be set (can be 'root' if desired)");
        }

        if self.auth.oidc.as_ref().is_some_and(|oidc| oidc.enabled) {
            let _ = self.server.oidc_callback_url()?;
        }

        match self.commands.signal_transport {
            SignalTransportMode::Cli => {
                if self.commands.signal_cli_path.trim().is_empty() {
                    anyhow::bail!(
                        "commands.signal_cli_path must not be empty when commands.signal_transport=cli"
                    );
                }
            }
            SignalTransportMode::Api => {
                let api_url = self.commands.signal_api_url.trim();
                if api_url.is_empty() {
                    anyhow::bail!(
                        "commands.signal_api_url must not be empty when commands.signal_transport=api"
                    );
                }
                Url::parse(api_url)
                    .with_context(|| format!("Invalid commands.signal_api_url: {api_url}"))?;
            }
        }

        let mcp_path = self.mcp.path.trim();
        if mcp_path.is_empty() {
            anyhow::bail!("mcp.path must not be empty");
        }
        if !mcp_path.starts_with('/') {
            anyhow::bail!("mcp.path must start with '/'");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, AuthConfig, CommandsConfig, DatabaseConfig, LoggingConfig, McpConfig,
        RemindersConfig, ServerConfig, SignalTransportMode,
    };

    fn server_config() -> ServerConfig {
        ServerConfig {
            listen: "0.0.0.0:3000".to_string(),
            base_url: None,
            server_name: Some("birthdays.example.com".to_string()),
            scheme: "https".to_string(),
            session_secret: "x".repeat(32),
            encryption_key: "key".to_string(),
            trusted_proxies: vec![],
            run_as_user: "birthday-reminders".to_string(),
            run_as_group: "birthday-reminders".to_string(),
        }
    }

    #[test]
    fn derives_public_base_url_from_server_name() {
        let config = server_config();

        assert_eq!(
            config.public_base_url().unwrap().as_str(),
            "https://birthdays.example.com/"
        );
    }

    #[test]
    fn explicit_base_url_overrides_derived_url() {
        let mut config = server_config();
        config.base_url = Some("https://public.example.com/app".to_string());

        assert_eq!(
            config.public_base_url().unwrap().as_str(),
            "https://public.example.com/app"
        );
        assert_eq!(
            config.oidc_callback_url().unwrap().as_str(),
            "https://public.example.com/app/auth/oidc/callback"
        );
    }

    #[test]
    fn rejects_missing_server_name_when_base_url_is_absent() {
        let mut config = server_config();
        config.server_name = None;

        assert!(config.public_base_url().is_err());
    }

    #[test]
    fn parses_trusted_proxy_addresses_and_cidrs() {
        let mut config = server_config();
        config.trusted_proxies = vec!["127.0.0.1".to_string(), "10.0.0.0/8".to_string()];

        let nets = config.trusted_proxy_nets().unwrap();

        assert_eq!(nets.len(), 2);
    }

    fn app_config() -> AppConfig {
        AppConfig {
            database: DatabaseConfig {
                url: "sqlite:///tmp/test.db?mode=rwc".to_string(),
                max_connections: 10,
            },
            server: server_config(),
            auth: AuthConfig {
                allow_registration: false,
                oidc: None,
            },
            reminders: RemindersConfig {
                schedule: "0 0 8 * * *".to_string(),
                default_days_before: vec![7, 3, 1, 0],
            },
            mcp: McpConfig::default(),
            commands: CommandsConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

    #[test]
    fn commands_config_defaults_signal_cli_path() {
        let config: AppConfig = serde_yaml::from_str(
            r#"
database:
  url: "sqlite:///tmp/test.db?mode=rwc"
server:
  listen: "0.0.0.0:3000"
  server_name: "birthdays.example.com"
  session_secret: "12345678901234567890123456789012"
  encryption_key: "enc-key"
  run_as_user: "birthday-reminders"
  run_as_group: "birthday-reminders"
auth: {}
reminders: {}
mcp: {}
logging: {}
"#,
        )
        .unwrap();

        assert_eq!(config.commands.signal_transport, SignalTransportMode::Cli);
        assert_eq!(config.commands.signal_cli_path, "signal-cli");
        assert_eq!(config.commands.signal_api_url, "http://127.0.0.1:8080");
        assert!(!config.mcp.enabled);
        assert_eq!(config.mcp.path, "/mcp");
    }

    #[test]
    fn rejects_empty_signal_cli_path() {
        let mut config = app_config();
        config.commands.signal_cli_path = "   ".to_string();

        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_invalid_signal_api_url_in_api_mode() {
        let mut config = app_config();
        config.commands.signal_transport = SignalTransportMode::Api;
        config.commands.signal_api_url = "not-a-url".to_string();

        assert!(config.validate().is_err());
    }
}
