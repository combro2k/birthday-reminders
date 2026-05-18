//! OpenAPI 3.1 specification served at `GET /openapi.json`.
//!
//! Schemas and path metadata are generated at compile time via `utoipa` so the
//! document tracks the application version and benefits from `utoipa` updates.
//! The handler patches the MCP path at runtime to reflect the configured
//! [`McpConfig::path`] and omits the MCP entry entirely when MCP is disabled.

use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use super::server::AppState;

/// Response body returned by the `/health` endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Overall service status.
    #[schema(example = "ok")]
    pub status: String,
    /// Failure reason, present only when `status` is `unhealthy`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// JSON-RPC 2.0 request envelope accepted by the MCP endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct McpJsonRpcRequest {
    /// Always `"2.0"`.
    #[schema(example = "2.0")]
    pub jsonrpc: String,
    /// Optional request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    /// MCP method name (e.g. `tools/list`, `tools/call`).
    pub method: String,
    /// Method-specific parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response envelope returned by the MCP endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct McpJsonRpcResponse {
    /// Always `"2.0"`.
    #[schema(example = "2.0")]
    pub jsonrpc: String,
    /// Echoes the request `id` (or `null` for notifications).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    /// Method result on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error object on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Path documentation stubs.
//
// These functions are never called. They exist purely to attach
// `#[utoipa::path]` metadata to a stable symbol, so the real handlers (which
// live in unrelated modules, or are provided by `rmcp`) stay free of doc
// macros.
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/health",
    tag = "infrastructure",
    security(),
    responses(
        (status = 200, description = "Service is healthy.", body = HealthResponse),
        (status = 503, description = "Service is unhealthy (e.g. database unreachable).", body = HealthResponse)
    )
)]
#[allow(dead_code)]
fn health_doc() {}

#[utoipa::path(
    get,
    path = "/openapi.json",
    tag = "infrastructure",
    responses(
        (status = 200, description = "OpenAPI specification document.", content_type = "application/json"),
        (status = 401, description = "Authentication required.")
    )
)]
#[allow(dead_code)]
fn openapi_doc() {}

#[utoipa::path(
    post,
    path = "/mcp",
    tag = "mcp",
    security(("bearerAuth" = [])),
    request_body = McpJsonRpcRequest,
    responses(
        (status = 200, description = "JSON-RPC response, optionally streamed as Server-Sent Events.", body = McpJsonRpcResponse),
        (status = 401, description = "Missing or invalid Bearer token.")
    )
)]
#[allow(dead_code)]
fn mcp_doc() {}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Birthday Reminders API",
        description = "JSON endpoints exposed by Birthday Reminders. HTML/form routes are not documented here.",
        license(name = "MIT")
    ),
    paths(health_doc, openapi_doc, mcp_doc),
    components(schemas(HealthResponse, McpJsonRpcRequest, McpJsonRpcResponse)),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{
            ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme,
        };

        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::new);

        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some("API token issued via /settings/api-tokens."))
                    .build(),
            ),
        );
        components.add_security_scheme(
            "sessionCookie",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
                "id",
                "Browser session cookie set after login.",
            ))),
        );
    }
}

/// Build the OpenAPI document, applying runtime config (MCP enablement and path).
fn build_spec(state: &AppState) -> utoipa::openapi::OpenApi {
    let mut spec = ApiDoc::openapi();
    spec.info.version = env!("CARGO_PKG_VERSION").to_string();

    let mcp_enabled = state.config.mcp.enabled;
    let configured_path = state.config.mcp.path.clone();

    if !mcp_enabled {
        spec.paths.paths.remove("/mcp");
    } else if configured_path != "/mcp"
        && let Some(item) = spec.paths.paths.remove("/mcp")
    {
        spec.paths.paths.insert(configured_path, item);
    }

    spec
}

/// Handler for `GET /openapi.json`. Registered behind the auth middleware.
pub async fn openapi_spec_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(build_spec(state.as_ref()))
}
