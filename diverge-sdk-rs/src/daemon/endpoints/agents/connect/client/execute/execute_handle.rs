//! A connected agent, and everything a client can do as it.

use std::fmt;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};
use tokio::sync::Mutex;

use super::super::channel_request;
use crate::provider::endpoints::containers::client::answered::{McpCallTool, McpListResources, McpListTools, McpNotifications, McpReadResource};
use crate::provider::endpoints::containers::client::{ChannelStream, OpenError, UnaryError, unary};
use crate::shared::mcp;
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// The agent's tools' notifications, for as long as the channel lives.
pub type McpNotificationsStream = ChannelStream<McpNotifications>;

/// The scope a connect opened, held for the connection's life.
///
/// Every channel a client may open on a connected agent is a method
/// here: the five MCP exchanges, and the disconnect. Each opens its
/// own channel, so several may be in flight at once. The readers are
/// the provider protocol's: the same shared frames answer the same
/// five exchanges there, and the machinery that reads them is written
/// once.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame. Ending a connect is
/// [`disconnect`](Self::disconnect), or the connection going; the
/// daemon finishes the scope on either.
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
    main: Mutex<crate::wire::credit::Receiver>,
}

/// One channel request of this scope, as its bytes.
fn payload(frame: &channel_request::Frame) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes))?;
    Ok(bytes)
}

impl ExecuteHandle {
    pub(super) fn new(handle: Handle, scope: u32, main: crate::wire::credit::Receiver) -> Self {
        ExecuteHandle {
            handle,
            scope,
            main: Mutex::new(main),
        }
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// `tools/list` as the agent: every tool served to it.
    pub async fn list_tools(&self, params: Option<PaginatedRequestParams>) -> Result<ListToolsResult, UnaryError<McpListTools>> {
        let payload = payload(&channel_request::Frame::McpListTools(mcp::list_tools::request::Request(params))).map_err(UnaryError::Request)?;
        unary::<McpListTools>(&self.handle, self.scope, &payload).await
    }

    /// `resources/list` as the agent.
    pub async fn list_resources(&self, params: Option<PaginatedRequestParams>) -> Result<ListResourcesResult, UnaryError<McpListResources>> {
        let payload =
            payload(&channel_request::Frame::McpListResources(mcp::list_resources::request::Request(params))).map_err(UnaryError::Request)?;
        unary::<McpListResources>(&self.handle, self.scope, &payload).await
    }

    /// `tools/call` as the agent, by the name it sees the tool by.
    pub async fn call_tool(&self, params: CallToolRequestParams) -> Result<CallToolResult, UnaryError<McpCallTool>> {
        let payload = payload(&channel_request::Frame::McpCallTool(mcp::call_tool::request::Request(params))).map_err(UnaryError::Request)?;
        unary::<McpCallTool>(&self.handle, self.scope, &payload).await
    }

    /// `resources/read` as the agent.
    pub async fn read_resource(&self, params: ReadResourceRequestParams) -> Result<ReadResourceResult, UnaryError<McpReadResource>> {
        let payload =
            payload(&channel_request::Frame::McpReadResource(mcp::read_resource::request::Request(params))).map_err(UnaryError::Request)?;
        unary::<McpReadResource>(&self.handle, self.scope, &payload).await
    }

    /// Everything the agent's tools say on their own account, for as
    /// long as the channel lives.
    pub async fn notifications(&self) -> Result<McpNotificationsStream, OpenError> {
        let payload = payload(&channel_request::Frame::McpNotifications(mcp::notifications::request::Request)).map_err(OpenError::Request)?;
        let channel = self.handle.send_channel_request(self.scope, &payload).await.map_err(OpenError::Send)?;
        Ok(ChannelStream::new(channel.response_receiver))
    }

    /// Leave the agent. Nothing answers on the channel this opens; what
    /// answers is the scope's own finish, on [`wait`](Self::wait).
    pub async fn disconnect(&self) -> Result<(), OpenError> {
        let payload = payload(&channel_request::Frame::Disconnect).map_err(OpenError::Request)?;
        self.handle.send_channel_request(self.scope, &payload).await.map(|_| ()).map_err(OpenError::Send)
    }

    /// The connection's end: the finish, after the disconnect, or the
    /// connection going first. Resolves once
    /// and answers the same way again after.
    pub async fn wait(&self) -> End {
        let mut main = self.main.lock().await;
        while let Some(bytes) = main.recv().await {
            match frame::server::ServerFrame::decode(&bytes) {
                Ok(frame::server::ServerFrame::ResponseFinish { .. }) => return End::Finished,
                Ok(_) => {}
                Err(_) => return End::Closed,
            }
        }
        End::Closed
    }
}

/// How a connect ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The finish came: the disconnect was answered.
    Finished,
    /// The connection ended before the finish.
    Closed,
}

impl fmt::Debug for ExecuteHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecuteHandle").field("scope", &self.scope).finish_non_exhaustive()
    }
}
