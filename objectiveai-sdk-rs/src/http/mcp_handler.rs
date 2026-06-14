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

/// JSON-RPC error code for the reject-handler's stock reply. -32601 is
/// "method not found"; pairs cleanly with the proxy seeing the API
/// fall through to a fallback agent.
const REJECT_CODE: i64 = -32601;
const REJECT_MESSAGE: &str = "this client does not host objectiveai-mcp";

fn reject_err<R>() -> server_response::JsonRpcResult<R> {
    server_response::JsonRpcResult::Err {
        code: REJECT_CODE,
        message: REJECT_MESSAGE.into(),
        data: None,
    }
}

impl McpHandler for RejectHandler {
    async fn handle(
        &self,
        request: server_request::Request,
    ) -> server_response::Response {
        // Reply with a typed `JsonRpcResult::Err` in the variant
        // that pairs with the inbound payload — the API's
        // `variant_mismatch` check is satisfied AND the error
        // surfaces as a method-not-found on the proxy side.
        use server_response::Payload;
        let payload = match request.payload {
            server_request::Payload::Initialize { mcp_kind, .. } => {
                Payload::Initialize { mcp_kind, result: reject_err() }
            }
            server_request::Payload::ToolsList { mcp_kind, .. } => {
                Payload::ToolsList { mcp_kind, result: reject_err() }
            }
            server_request::Payload::ToolsCall { mcp_kind, .. } => {
                Payload::ToolsCall { mcp_kind, result: reject_err() }
            }
            server_request::Payload::ResourcesList { mcp_kind, .. } => {
                Payload::ResourcesList { mcp_kind, result: reject_err() }
            }
            server_request::Payload::ResourcesRead { mcp_kind, .. } => {
                Payload::ResourcesRead { mcp_kind, result: reject_err() }
            }
            server_request::Payload::SessionTerminate { mcp_kind } => {
                Payload::SessionTerminate { mcp_kind, result: reject_err() }
            }
            server_request::Payload::ReadMessageQueue(_) => {
                Payload::ReadMessageQueue(reject_err())
            }
            server_request::Payload::Retrieve(_) => {
                Payload::Retrieve(reject_err())
            }
        };
        server_response::Response {
            id: request.id,
            payload,
        }
    }
}
