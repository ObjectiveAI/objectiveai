//! The MCP server the agent talks to — every method a relay.

use std::sync::Arc;

use bytes::Bytes;
use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::channel_request;
use diverge_provider_sdk::shared::mcp;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ServerCapabilities, ServerInfo,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler};

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
}

impl ProxyHandler {
    /// One ask, one answer: open the channel, take the first
    /// response, decode it as the exchange's response frame.
    async fn exchange<F>(
        &self,
        request: channel_request::Frame,
    ) -> Result<F, ErrorData>
    where
        F: for<'a> Decode<'a>,
    {
        let bytes = self.ask(request).await?;
        F::decode(&bytes).map_err(|_| unreadable())
    }

    async fn ask(
        &self,
        request: channel_request::Frame,
    ) -> Result<Bytes, ErrorData> {
        let mut receiver = self
            .proxy
            .open(request)
            .await
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        // The stream ending with nothing on it is the far side saying
        // the exchange could not be served at all — or the connection
        // going. The agent is told the same thing either way, because
        // from inside the container they are the same fact.
        receiver.recv().await.ok_or_else(unanswered)
    }
}

impl ServerHandler for ProxyHandler {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .build();
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

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        match self
            .exchange(channel_request::Frame::McpCallTool(
                mcp::call_tool::request::Request(request),
            ))
            .await?
        {
            mcp::call_tool::response::Frame::Result(result) => {
                Ok(result.into())
            }
            mcp::call_tool::response::Frame::Error(error) => Err(error),
        }
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

    /// The fifth exchange, opened when a session begins: the far
    /// side's notifications, relayed to this session's peer for as
    /// long as both live.
    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        let proxy = Arc::clone(&self.proxy);
        let peer = context.peer.clone();
        tokio::spawn(async move {
            let Ok(mut receiver) = proxy
                .open(channel_request::Frame::McpNotifications(
                    mcp::notifications::request::Request,
                ))
                .await
            else {
                return;
            };
            while let Some(bytes) = receiver.recv().await {
                match mcp::notifications::response::Frame::decode(&bytes) {
                    Ok(mcp::notifications::response::Frame::Notification(
                        notification,
                    )) => {
                        if peer.send_notification(notification).await.is_err() {
                            // The session is gone; there is nobody
                            // left to relay to.
                            return;
                        }
                    }
                    // The far side saying it will push no more, and
                    // why — or a frame this crate could not read.
                    // Either ends the relay; an agent reading a stream
                    // that stops is the ordinary ending here.
                    Ok(mcp::notifications::response::Frame::Error(_))
                    | Err(_) => return,
                }
            }
        });
    }
}

/// Nobody answered, and the agent has to be told something.
fn unanswered() -> ErrorData {
    ErrorData::internal_error("the caller did not answer", None)
}

/// Something answered and this crate could not read it.
fn unreadable() -> ErrorData {
    ErrorData::internal_error("the caller's answer could not be read", None)
}
