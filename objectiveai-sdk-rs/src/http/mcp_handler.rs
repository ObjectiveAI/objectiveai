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
        // Reply with a typed `JsonRpcResult::Err` in the variant
        // that pairs with the inbound payload — the API's
        // `variant_mismatch` check is satisfied AND the error
        // surfaces as a method-not-found on the proxy side.
        use server_response::{JsonRpcResult, Payload};
        const CODE: i64 = -32601;
        const MESSAGE: &str = "this client does not host objectiveai-mcp";
        let payload = match request.payload {
            server_request::Payload::Initialize(_) => Payload::Initialize(JsonRpcResult::Err {
                code: CODE,
                message: MESSAGE.into(),
                data: None,
            }),
            server_request::Payload::ToolsList(_) => Payload::ToolsList(JsonRpcResult::Err {
                code: CODE,
                message: MESSAGE.into(),
                data: None,
            }),
            server_request::Payload::ToolsCall(_) => Payload::ToolsCall(JsonRpcResult::Err {
                code: CODE,
                message: MESSAGE.into(),
                data: None,
            }),
            server_request::Payload::ResourcesList(_) => Payload::ResourcesList(JsonRpcResult::Err {
                code: CODE,
                message: MESSAGE.into(),
                data: None,
            }),
            server_request::Payload::ResourcesRead(_) => Payload::ResourcesRead(JsonRpcResult::Err {
                code: CODE,
                message: MESSAGE.into(),
                data: None,
            }),
            server_request::Payload::SessionTerminate => Payload::SessionTerminate(JsonRpcResult::Err {
                code: CODE,
                message: MESSAGE.into(),
                data: None,
            }),
        };
        server_response::Response {
            id: request.id,
            mcp_kind: request.mcp_kind,
            payload,
        }
    }
}
