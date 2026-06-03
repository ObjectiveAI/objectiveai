//! Handler trait for inbound objectiveai-mcp `server_request` frames.
//!
//! The API hosts `/objectiveai-mcp/{session_id}` as a pure HTTP→WS
//! bridge: every HTTP request the proxy makes is forwarded over the
//! reverse-attach WS as a [`server_request::Request`], and the
//! handler's returned [`server_response::Response`] is translated
//! back into the HTTP response the proxy receives.
//!
//! Clients that host objectiveai-mcp (e.g. the CLI) implement
//! [`McpHandler`] to spawn / forward / reply. Clients that don't
//! use [`RejectHandler`], which 501s every request — the API's
//! list-tools verification probe then short-circuits and any agent
//! that declares `client_objectiveai_mcp` falls through to the next
//! fallback server-side.

use crate::client_objectiveai_mcp::{server_request, server_response};
use std::future::Future;

/// Handler for inbound `server_request` frames on a streaming WS.
///
/// One handler instance is bound at `create_streaming` time and
/// stays live for the lifetime of the WS session. Implementations
/// must be `Send + Sync + 'static` since the demux task that
/// invokes them is spawned.
pub trait McpHandler: Send + Sync + 'static {
    /// Dispatch a single request. The returned `Response`'s `id`
    /// must echo `request.id` so the API can correlate the reply
    /// to the in-flight proxy request waiting on it.
    fn handle(
        &self,
        request: server_request::Request,
    ) -> impl Future<Output = server_response::Response> + Send;
}

/// Default handler that 501s every `server_request`. Used when the
/// calling client doesn't host objectiveai-mcp — agents that
/// declare `client_objectiveai_mcp` will see this and fall through
/// to the next fallback agent on the server side.
#[derive(Debug, Clone, Copy, Default)]
pub struct RejectHandler;

impl McpHandler for RejectHandler {
    async fn handle(
        &self,
        request: server_request::Request,
    ) -> server_response::Response {
        server_response::Response {
            id: request.id,
            status: 501,
            headers: indexmap::IndexMap::new(),
            body: Some(serde_json::json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "error": {
                    "code": -32601,
                    "message": "this client does not host objectiveai-mcp",
                },
            })),
        }
    }
}
