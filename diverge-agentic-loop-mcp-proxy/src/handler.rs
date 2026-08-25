//! The MCP server the agent talks to — every method a relay.

use std::sync::Arc;

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

use crate::proxy::{ChannelEvent, Proxy};

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
    /// side's notifications, relayed to this session's peer.
    ///
    /// Notifications are emitted as they arrive — before any finish —
    /// and the relay outlives connections: a connection dying does
    /// not end it, it re-opens the channel on the next connection and
    /// keeps propagating. What DOES end it is deliberate — the far
    /// side's error frame or clean finish, an answer this crate could
    /// not read, or the session itself going away.
    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        let proxy = Arc::clone(&self.proxy);
        let peer = context.peer.clone();
        tokio::spawn(async move {
            loop {
                let Ok(mut receiver) = proxy
                    .open(channel_request::Frame::McpNotifications(
                        mcp::notifications::request::Request,
                    ))
                    .await
                else {
                    // The request would not encode, which cannot
                    // change by retrying: it has no params.
                    return;
                };
                while let Some(event) = receiver.recv().await {
                    let bytes = match event {
                        ChannelEvent::Response(bytes) => bytes,
                        // The far side closing the stream on purpose.
                        // Nothing follows a finish, and nothing is
                        // re-asked: the stream ended, it did not die.
                        ChannelEvent::Finished => return,
                    };
                    match mcp::notifications::response::Frame::decode(&bytes) {
                        Ok(mcp::notifications::response::Frame::Notification(
                            notification,
                        )) => {
                            if peer
                                .send_notification(notification)
                                .await
                                .is_err()
                            {
                                // The session is gone; there is
                                // nobody left to relay to.
                                return;
                            }
                        }
                        // The far side saying it will push no more,
                        // and why — or a frame this crate could not
                        // read. Both are deliberate endings, not
                        // deaths.
                        Ok(mcp::notifications::response::Frame::Error(_))
                        | Err(_) => return,
                    }
                }
                // The stream ended un-finished: the connection died
                // under it. The next iteration re-opens on the next
                // connection, however long that takes.
            }
        });
    }
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
