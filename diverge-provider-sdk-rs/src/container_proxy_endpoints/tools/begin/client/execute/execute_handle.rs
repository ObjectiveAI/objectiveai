//! The begin scope, held for the connection's life.

use serde_json::Value;

use super::super::channel_request;
use crate::client::handle::{Handle, SendError};
use crate::encode::{Encode, Writer};
use crate::endpoints::containers::client::answered::{
    McpCallTool, McpListResources, McpListTools, McpNotifications, McpReadResource, Postgres, Schema,
};
use crate::endpoints::containers::client::{Answered, ChannelStream, OpenError, UnaryError, unary};
use crate::shared::containers::postgres;
use crate::shared::mcp;

/// The begin scope a tool container's server holds.
///
/// Every channel the server may open on it is a method here — the
/// arguments' schema, the five MCP exchanges into the container's
/// server, the server's half of a database connection — and so is answering a channel the proxy
/// opened, by the proxy's own channel number. Clones share the scope,
/// which is how a connector's exchanges ride the runner's begin.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame. The scope is the connection's life,
/// and the connection ends when the container does.
#[derive(Debug, Clone)]
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
}

/// One channel request of this family, as its bytes.
fn payload(frame: &channel_request::Frame) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes))?;
    Ok(bytes)
}

impl ExecuteHandle {
    pub(super) fn new(handle: Handle, scope: u32) -> Self {
        ExecuteHandle { handle, scope }
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// What the arguments may be: the image's JSON Schema for them.
    pub async fn schema(&self) -> Result<Value, UnaryError<Schema>> {
        let payload = payload(&channel_request::Frame::Schema).map_err(UnaryError::Request)?;
        unary::<Schema>(&self.handle, self.scope, &payload).await
    }

    /// What tools the container's server has.
    pub async fn list_tools(
        &self,
        request: mcp::list_tools::request::Request,
    ) -> Result<<McpListTools as Answered>::Item, UnaryError<McpListTools>> {
        let payload = payload(&channel_request::Frame::McpListTools(request)).map_err(UnaryError::Request)?;
        unary::<McpListTools>(&self.handle, self.scope, &payload).await
    }

    /// What resources it has.
    pub async fn list_resources(
        &self,
        request: mcp::list_resources::request::Request,
    ) -> Result<<McpListResources as Answered>::Item, UnaryError<McpListResources>> {
        let payload = payload(&channel_request::Frame::McpListResources(request)).map_err(UnaryError::Request)?;
        unary::<McpListResources>(&self.handle, self.scope, &payload).await
    }

    /// Run one of its tools.
    pub async fn call_tool(
        &self,
        request: mcp::call_tool::request::Request,
    ) -> Result<<McpCallTool as Answered>::Item, UnaryError<McpCallTool>> {
        let payload = payload(&channel_request::Frame::McpCallTool(request)).map_err(UnaryError::Request)?;
        unary::<McpCallTool>(&self.handle, self.scope, &payload).await
    }

    /// Read one of its resources.
    pub async fn read_resource(
        &self,
        request: mcp::read_resource::request::Request,
    ) -> Result<<McpReadResource as Answered>::Item, UnaryError<McpReadResource>> {
        let payload = payload(&channel_request::Frame::McpReadResource(request)).map_err(UnaryError::Request)?;
        unary::<McpReadResource>(&self.handle, self.scope, &payload).await
    }

    /// Everything it says on its own account, for as long as the
    /// channel lives.
    pub async fn notifications(&self) -> Result<ChannelStream<McpNotifications>, OpenError> {
        let payload = payload(&channel_request::Frame::McpNotifications(
            mcp::notifications::request::Request,
        ))
        .map_err(OpenError::Request)?;
        let channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(OpenError::Send)?;
        Ok(ChannelStream::new(channel.response_receiver))
    }

    /// The server's half of a database connection the proxy
    /// announced, quoting the id the proxy minted: what comes back is
    /// everything the container's driver wrote, until the finish.
    pub async fn postgres(&self, connection_id: u32) -> Result<ChannelStream<Postgres>, OpenError> {
        let payload = payload(&channel_request::Frame::Postgres(postgres::request::Postgres { connection_id }))
            .map_err(OpenError::Request)?;
        let channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(OpenError::Send)?;
        Ok(ChannelStream::new(channel.response_receiver))
    }

    /// One answer on a channel the proxy opened, by its number.
    pub async fn respond(&self, channel: u32, payload: &[u8]) -> Result<(), SendError> {
        self.handle.send_channel_response(self.scope, channel, payload).await
    }

    /// The end of the answers on a channel the proxy opened.
    pub async fn finish(&self, channel: u32) -> Result<(), SendError> {
        self.handle.send_channel_response_finish(self.scope, channel).await
    }
}
