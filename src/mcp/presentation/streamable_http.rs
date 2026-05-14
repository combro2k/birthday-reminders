use std::sync::Arc;

use rmcp::{
    ErrorData,
    handler::server::wrapper::Parameters,
    tool, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};

use crate::infrastructure::web::server::AppState;
use crate::mcp::application::birthdays::{
    AddBirthdayInput, ListBirthdaysInput, RemoveBirthdayInput, UpcomingBirthdaysInput,
};
use crate::mcp::application::setup_guide;

#[derive(Clone)]
pub(crate) struct BirthdayMcpServer {
    state: Arc<AppState>,
}

impl BirthdayMcpServer {
    fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tool_router(server_handler)]
impl BirthdayMcpServer {
    #[tool(description = "List all birthdays for the authenticated user. Token is mandatory.")]
    async fn list_birthdays(
        &self,
        Parameters(input): Parameters<ListBirthdaysInput>,
    ) -> Result<String, ErrorData> {
        crate::mcp::application::birthdays::list_birthdays(self.state.as_ref(), input).await
    }

    #[tool(
        description = "List upcoming birthdays for the authenticated user. Token is mandatory. Optional `days` defaults to 30."
    )]
    async fn upcoming_birthdays(
        &self,
        Parameters(input): Parameters<UpcomingBirthdaysInput>,
    ) -> Result<String, ErrorData> {
        crate::mcp::application::birthdays::upcoming_birthdays(self.state.as_ref(), input).await
    }

    #[tool(
        description = "Add a birthday for the authenticated user. Token is mandatory. `birth_date` must be YYYY-MM-DD."
    )]
    async fn add_birthday(
        &self,
        Parameters(input): Parameters<AddBirthdayInput>,
    ) -> Result<String, ErrorData> {
        crate::mcp::application::birthdays::add_birthday(self.state.as_ref(), input).await
    }

    #[tool(
        description = "Remove birthday is intentionally not supported in MCP. Token is mandatory; use the web interface for deletion."
    )]
    async fn remove_birthday(
        &self,
        Parameters(input): Parameters<RemoveBirthdayInput>,
    ) -> Result<String, ErrorData> {
        crate::mcp::application::birthdays::remove_birthday_not_supported(
            self.state.as_ref(),
            input,
        )
        .await
    }

    #[tool(
        description = "Get MCP setup guide with configuration instructions for LM Studio, Hermes, Claude Desktop, Cursor, and other clients. No authentication required."
    )]
    async fn get_mcp_setup_guide(&self) -> Result<String, ErrorData> {
        setup_guide::get_mcp_setup_guide().await
    }
}

pub fn build_streamable_http_service(
    state: Arc<AppState>,
) -> StreamableHttpService<BirthdayMcpServer, LocalSessionManager> {
    let config = build_transport_config(state.as_ref());

    StreamableHttpService::new(
        {
            let state = state.clone();
            move || Ok(BirthdayMcpServer::new(state.clone()))
        },
        Arc::new(LocalSessionManager::default()),
        config,
    )
}

fn build_transport_config(state: &AppState) -> StreamableHttpServerConfig {
    let mcp_config = &state.config.mcp;
    let mut config = StreamableHttpServerConfig::default()
        .with_stateful_mode(mcp_config.stateful_mode)
        .with_json_response(mcp_config.json_response);

    let mut allowed_hosts = if mcp_config.allowed_hosts.is_empty() {
        vec![
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            "::1".to_string(),
        ]
    } else {
        mcp_config.allowed_hosts.clone()
    };

    if let Some(server_name) = state.config.server.server_name.as_ref() {
        let trimmed = server_name.trim();
        if !trimmed.is_empty() {
            allowed_hosts.push(trimmed.to_string());
        }
    }

    if let Some(base_url) = state.config.server.base_url.as_ref()
        && let Ok(url) = url::Url::parse(base_url)
        && let Some(host) = url.host_str()
    {
        if let Some(port) = url.port() {
            allowed_hosts.push(format!("{}:{}", host, port));
        }
        allowed_hosts.push(host.to_string());
    }

    allowed_hosts.sort();
    allowed_hosts.dedup();
    config = config.with_allowed_hosts(allowed_hosts);

    if !mcp_config.allowed_origins.is_empty() {
        config = config.with_allowed_origins(mcp_config.allowed_origins.clone());
    }

    config
}
