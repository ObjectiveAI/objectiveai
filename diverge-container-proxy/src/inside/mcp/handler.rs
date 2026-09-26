//! The MCP server the agent talks to — every method a relay.

use std::sync::Arc;

use diverge_sdk::wire::decode::Decode;
use diverge_sdk::shared::mcp;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities,
    ServerInfo,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::ask::{self, Asked};
use crate::own::Own;
use crate::proxy::{Begun, Proxy};

/// A compliant MCP server whose answers all live somewhere else.
///
/// Each method puts the container's image under the params' `_meta`
/// — with the program's own `_meta`, which rmcp took off the wire
/// into the context — sends its exchange as one of the proxy's own
/// asks on the begin scope, and decodes the one frame that answers
/// it. The
/// errors an MCP server on the far side sent travel through as
/// themselves — their JSON-RPC codes are content — and the failures
/// of the relay itself arrive as internal errors in the same
/// vocabulary.
#[derive(Clone)]
pub struct Handler {
    proxy: Arc<Proxy>,
}

impl Handler {
    pub fn new(proxy: Arc<Proxy>) -> Self {
        Handler { proxy }
    }

    /// One ask, one answer.
    ///
    /// An ask is answered when one frame arrived AND its channel
    /// finished. The failures are the ones no retry could cure, and
    /// there is no connection to retry on: the far side's deliberate
    /// empty finish, an answer this crate could not read, an ask that
    /// would not encode, and the connection gone.
    async fn exchange<F>(&self, own: Own<'static>) -> Result<F, ErrorData>
    where
        F: for<'a> Decode<'a>,
    {
        self.proxy.gate.open();
        let bytes = ask::own(&self.proxy, own).await.map_err(|asked| match asked {
            Asked::Encode => ErrorData::internal_error("the exchange did not encode", None),
            Asked::Empty => unserved(),
            Asked::Died => gone(),
        })?;
        F::decode(&bytes).map_err(|_| unreadable())
    }

    /// The begin, waited for as every ask waits for it: the stamp is
    /// its, and a proxy without one is a proxy whose connection is
    /// gone.
    async fn begun(&self) -> Result<Begun, ErrorData> {
        self.proxy.begun().await.ok_or_else(gone)
    }
}

impl ServerHandler for Handler {
    /// Tools, resources, and the list-changed notifications the
    /// resident stream broadcasts — advertised, because a client that
    /// was not promised a notification is entitled to ignore it.
    /// `resources.subscribe` is deliberately absent: `resources/
    /// subscribe` is not one of the wire's exchanges, so it is a
    /// promise this server could not keep.
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_tool_list_changed()
            .enable_resources()
            .enable_resources_list_changed()
            .build();
        // Expanded HERE, not in rmcp: `env!` reads the crate being
        // compiled, so the default introduces every server as "rmcp".
        info.server_info =
            Implementation::new("diverge", env!("CARGO_PKG_VERSION"));
        info
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut request = request.unwrap_or_default();
        self.begun().await?.stamp.request(&mut request, &context.meta);
        match self
            .exchange(Own::McpListTools(
                mcp::list_tools::request::Request(Some(request)),
            ))
            .await?
        {
            mcp::list_tools::response::Frame::Result(result) => Ok(result),
            mcp::list_tools::response::Frame::Error(error) => Err(error),
        }
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut request = request.unwrap_or_default();
        self.begun().await?.stamp.request(&mut request, &context.meta);
        match self
            .exchange(Own::McpListResources(
                mcp::list_resources::request::Request(Some(request)),
            ))
            .await?
        {
            mcp::list_resources::response::Frame::Result(result) => Ok(result),
            mcp::list_resources::response::Frame::Error(error) => Err(error),
        }
    }

    /// The one method that never returns `Err(ErrorData)`: every
    /// `ErrorData` — the far side's refusal and the relay's own
    /// unanswered/unreadable — becomes a tool-level error result the
    /// agent can read and react to. An `Err` out of `call_tool` is
    /// thereby reserved for the MCP link itself failing, which is a
    /// distinction the loop on the other side depends on.
    async fn call_tool(
        &self,
        mut request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let begun = match self.begun().await {
            Ok(begun) => begun,
            Err(error) => return Ok(failed(error).into()),
        };
        begun.stamp.request(&mut request, &context.meta);
        let result = match self
            .exchange(Own::McpCallTool(
                mcp::call_tool::request::Request(request),
            ))
            .await
        {
            Ok(mcp::call_tool::response::Frame::Result(result)) => result,
            Ok(mcp::call_tool::response::Frame::Error(error)) => failed(error),
            Err(error) => failed(error),
        };
        Ok(result.into())
    }

    async fn read_resource(
        &self,
        mut request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        self.begun().await?.stamp.request(&mut request, &context.meta);
        match self
            .exchange(Own::McpReadResource(
                mcp::read_resource::request::Request(request),
            ))
            .await?
        {
            mcp::read_resource::response::Frame::Result(result) => {
                Ok(result.into())
            }
            mcp::read_resource::response::Frame::Error(error) => Err(error),
        }
    }

    /// A session that initialized is a session the resident
    /// notifications stream addresses from now on — registration is
    /// the whole of what happens here. The stream itself is one per
    /// PROXY, not per session: see [`notifications`](fn@super::notifications).
    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        self.proxy.peers.insert(context.peer.clone()).await;
    }
}

/// An `ErrorData`, as the tool-level error result it becomes: the
/// whole error as JSON text, `is_error` set, so nothing the far side
/// said is lost on the way to the agent.
fn failed(error: ErrorData) -> rmcp::model::CallToolResult {
    let text = serde_json::to_string(&error)
        .unwrap_or_else(|_| error.to_string());
    rmcp::model::CallToolResult::error(vec![
        rmcp::model::ContentBlock::text(text),
    ])
}

/// The far side's deliberate empty finish: the exchange could not be
/// served.
fn unserved() -> ErrorData {
    ErrorData::internal_error("the caller could not serve the exchange", None)
}

/// Something answered and this crate could not read it.
fn unreadable() -> ErrorData {
    ErrorData::internal_error("the caller's answer could not be read", None)
}

/// The connection ended with the exchange open: the caller is gone,
/// and so is the container, shortly.
fn gone() -> ErrorData {
    ErrorData::internal_error("the caller went away", None)
}
