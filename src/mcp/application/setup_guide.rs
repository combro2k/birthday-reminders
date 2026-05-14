use rmcp::ErrorData;
use serde::Serialize;

const MCP_SETUP_GUIDE: &str = include_str!("../../../SKILLS.md");

#[derive(Debug, Serialize)]
pub struct SetupGuideResponse {
    pub guide: String,
    pub endpoint_path: String,
    pub endpoint_description: String,
}

pub async fn get_mcp_setup_guide() -> Result<String, ErrorData> {
    let response = SetupGuideResponse {
        guide: MCP_SETUP_GUIDE.to_string(),
        endpoint_path: "/mcp".to_string(),
        endpoint_description: "Streamable HTTP MCP endpoint for Birthday Reminders".to_string(),
    };

    serde_json::to_string(&response).map_err(|e| {
        ErrorData::internal_error(format!("Failed to serialize setup guide: {e}"), None)
    })
}
