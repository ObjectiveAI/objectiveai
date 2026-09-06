//! The MCP server the agent talks to — every method a relay.

use std::sync::Arc;

use bytes::Bytes;
use diverge_provider_sdk::container_proxy::requests::request::Request;
use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::shared::mcp;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities,
    ServerInfo,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler};

use super::{Gate, Peers};
use crate::requests::{Event, Requests};

/// A compliant MCP server whose answers all live somewhere else.
///
/// Each method sends its exchange as an ask on `/requests` and
/// decodes the one message that answers it on the exchange's own
/// path. The errors an MCP server on the far side sent travel
/// through as themselves — their JSON-RPC codes are content — and
/// the failures of the relay itself arrive as internal errors in the
/// same vocabulary.
#[derive(Clone)]
pub struct Handler {
    pub requests: Arc<Requests>,
    /// The broadcast registry the resident notifications stream fans
    /// out to; a session enters it when it initializes.
    pub peers: Arc<Peers>,
    /// What the notifications ask waits behind: opened by the first
    /// exchange that passes through [`exchange`](Self::exchange).
    pub gate: Arc<Gate>,
}

impl Handler {
    /// One logical ask, answered however many connections it takes.
    ///
    /// An ask is answered when one message arrived AND its path
    /// closed cleanly. One that died before that — `/requests` went
    /// before the path opened, or the path ended abruptly — is asked
    /// again, at-least-once accepted, because the agent is waiting
    /// and the alternative is telling it a transport story it can do
    /// nothing about. The failures that remain are the ones no retry
    /// can cure: the far side's deliberate empty close, an answer
    /// this crate could not read, and an ask that would not encode.
    async fn exchange<F>(&self, request: Request<'static>) -> Result<F, ErrorData>
    where
        F: for<'a> Decode<'a>,
    {
        self.gate.open();
        let bytes = loop {
            let (_, mut receiver) = self
                .requests
                .ask(request.clone())
                .await
                .map_err(|error| {
                    ErrorData::internal_error(error.to_string(), None)
                })?;

            let mut answer: Option<Bytes> = None;
            let outcome = loop {
                match receiver.recv().await {
                    // The first message is the answer; a unary path
                    // has no business carrying a second, and extras
                    // are ignored rather than obeyed.
                    Some(Event::Message(bytes)) => {
                        answer.get_or_insert(bytes);
                    }
                    Some(Event::Complete) => break Some(answer.take()),
                    Some(Event::Died) | None => break None,
                }
            };
            match outcome {
                Some(Some(bytes)) => break bytes,
                Some(None) => return Err(unserved()),
                None => continue,
            }
        };
        F::decode(&bytes).map_err(|_| unreadable())
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
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        match self
            .exchange(Request::McpListTools(
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
            .exchange(Request::McpListResources(
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
        let result = match self
            .exchange(Request::McpCallTool(
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
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        match self
            .exchange(Request::McpReadResource(
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

/// The far side's deliberate empty close: the exchange could not be
/// served, and retrying is refusing to hear that.
fn unserved() -> ErrorData {
    ErrorData::internal_error("the caller could not serve the exchange", None)
}

/// Something answered and this crate could not read it.
fn unreadable() -> ErrorData {
    ErrorData::internal_error("the caller's answer could not be read", None)
}
