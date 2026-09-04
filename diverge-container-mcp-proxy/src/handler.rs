//! The MCP server the agent talks to — every method a relay.

use std::sync::Arc;

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::channel_request;
use diverge_provider_sdk::shared::mcp;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities,
    ServerInfo,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::peers::Peers;
use crate::proxy::Proxy;

/// A compliant MCP server whose answers all live somewhere else.
///
/// Each method encodes its exchange, opens a channel on the proxy's
/// wire, and decodes the one frame that answers it. The errors an MCP
/// server on the far side sent travel through as themselves — their
/// JSON-RPC codes are content — and the failures of the relay itself
/// arrive as internal errors in the same vocabulary the SDK's own
/// relay uses.
#[derive(Clone)]
pub struct ProxyHandler {
    pub proxy: Arc<Proxy>,
    /// The broadcast registry the resident notifications stream fans
    /// out to; a session enters it when it initializes.
    pub peers: Arc<Peers>,
}

impl ProxyHandler {
    /// One logical ask, answered however many connections it takes:
    /// [`Proxy::ask`] retries the wire exchange until a response
    /// arrives and its channel finishes, and what comes back here is
    /// decoded as the exchange's response frame.
    ///
    /// The failures that remain are the ones no retry can cure: the
    /// far side's deliberate empty finish, an answer this crate could
    /// not read, and a request whose params would not serialize.
    async fn exchange<F>(
        &self,
        request: channel_request::Frame,
    ) -> Result<F, ErrorData>
    where
        F: for<'a> Decode<'a>,
    {
        let bytes = self
            .proxy
            .ask(request)
            .await
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?
            .ok_or_else(unserved)?;
        F::decode(&bytes).map_err(|_| unreadable())
    }
}

impl ServerHandler for ProxyHandler {
    /// Tools, resources, and the list-changed notifications the
    /// resident stream broadcasts — advertised, because a client that
    /// was not promised a notification is entitled to ignore it.
    /// `resources.subscribe` is deliberately absent: `resources/
    /// subscribe` is not one of the five wire exchanges, so it is a
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
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        match self
            .exchange(channel_request::Frame::McpListTools(
                mcp::list_tools::request::Request(request),
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
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        match self
            .exchange(channel_request::Frame::McpListResources(
                mcp::list_resources::request::Request(request),
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
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let mut result = match self
            .exchange(channel_request::Frame::McpCallTool(
                mcp::call_tool::request::Request(request),
            ))
            .await
        {
            Ok(mcp::call_tool::response::Frame::Result(result)) => result,
            Ok(mcp::call_tool::response::Frame::Error(error)) => {
                failed(error)
            }
            Err(error) => failed(error),
        };
        // The queue's seam: everything enqueued since the last tool
        // response rides in front of this one — error results
        // included, because the model reads those too. Downstream of
        // the retry law above, so a call re-asked across connection
        // deaths folds exactly once, at the moment the answer
        // actually goes to the agent.
        crate::queue::QUEUE.fold(&mut result).await;
        Ok(result.into())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        match self
            .exchange(channel_request::Frame::McpReadResource(
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
    /// PROXY, not per session: see [`crate::notifications`].
    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        self.peers.insert(context.peer.clone()).await;
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
/// served, and retrying is refusing to hear that.
fn unserved() -> ErrorData {
    ErrorData::internal_error("the caller could not serve the exchange", None)
}

/// Something answered and this crate could not read it.
fn unreadable() -> ErrorData {
    ErrorData::internal_error("the caller's answer could not be read", None)
}
