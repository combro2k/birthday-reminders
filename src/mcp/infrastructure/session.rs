use std::collections::HashMap;

use futures_core::Stream;
use rmcp::transport::streamable_http_server::session::{
    RestoreOutcome, SessionManager,
    local::{LocalSessionManager, LocalSessionManagerError},
};
use tokio::sync::RwLock;

use crate::users::domain::user::UserId;

/// Wraps [`LocalSessionManager`] and tracks which MCP session belongs to which
/// user. When a session is closed (HTTP DELETE or idle timeout), the user
/// binding is automatically removed.
pub struct AuthenticatedSessionManager {
    inner: LocalSessionManager,
    users: RwLock<HashMap<String, UserId>>,
}

impl Default for AuthenticatedSessionManager {
    fn default() -> Self {
        Self {
            inner: LocalSessionManager::default(),
            users: RwLock::new(HashMap::new()),
        }
    }
}

impl AuthenticatedSessionManager {
    /// Associate a user with an MCP session after successful Bearer
    /// authentication.
    pub async fn bind_user(&self, session_id: &str, user_id: UserId) {
        self.users
            .write()
            .await
            .insert(session_id.to_owned(), user_id);
    }

    /// Look up the user bound to an MCP session.
    pub async fn get_user(&self, session_id: &str) -> Option<UserId> {
        self.users.read().await.get(session_id).cloned()
    }
}

impl SessionManager for AuthenticatedSessionManager {
    type Error = LocalSessionManagerError;
    type Transport = <LocalSessionManager as SessionManager>::Transport;

    async fn create_session(
        &self,
    ) -> Result<
        (
            rmcp::transport::common::server_side_http::SessionId,
            Self::Transport,
        ),
        Self::Error,
    > {
        self.inner.create_session().await
    }

    async fn initialize_session(
        &self,
        id: &rmcp::transport::common::server_side_http::SessionId,
        message: rmcp::model::ClientJsonRpcMessage,
    ) -> Result<rmcp::model::ServerJsonRpcMessage, Self::Error> {
        self.inner.initialize_session(id, message).await
    }

    async fn has_session(
        &self,
        id: &rmcp::transport::common::server_side_http::SessionId,
    ) -> Result<bool, Self::Error> {
        self.inner.has_session(id).await
    }

    async fn close_session(
        &self,
        id: &rmcp::transport::common::server_side_http::SessionId,
    ) -> Result<(), Self::Error> {
        self.users.write().await.remove(id.as_ref());
        self.inner.close_session(id).await
    }

    async fn create_stream(
        &self,
        id: &rmcp::transport::common::server_side_http::SessionId,
        message: rmcp::model::ClientJsonRpcMessage,
    ) -> Result<
        impl Stream<Item = rmcp::transport::common::server_side_http::ServerSseMessage>
        + Send
        + Sync
        + 'static,
        Self::Error,
    > {
        self.inner.create_stream(id, message).await
    }

    async fn accept_message(
        &self,
        id: &rmcp::transport::common::server_side_http::SessionId,
        message: rmcp::model::ClientJsonRpcMessage,
    ) -> Result<(), Self::Error> {
        self.inner.accept_message(id, message).await
    }

    async fn create_standalone_stream(
        &self,
        id: &rmcp::transport::common::server_side_http::SessionId,
    ) -> Result<
        impl Stream<Item = rmcp::transport::common::server_side_http::ServerSseMessage>
        + Send
        + Sync
        + 'static,
        Self::Error,
    > {
        self.inner.create_standalone_stream(id).await
    }

    async fn resume(
        &self,
        id: &rmcp::transport::common::server_side_http::SessionId,
        last_event_id: String,
    ) -> Result<
        impl Stream<Item = rmcp::transport::common::server_side_http::ServerSseMessage>
        + Send
        + Sync
        + 'static,
        Self::Error,
    > {
        self.inner.resume(id, last_event_id).await
    }

    async fn restore_session(
        &self,
        id: rmcp::transport::common::server_side_http::SessionId,
    ) -> Result<RestoreOutcome<Self::Transport>, Self::Error> {
        self.inner.restore_session(id).await
    }
}
