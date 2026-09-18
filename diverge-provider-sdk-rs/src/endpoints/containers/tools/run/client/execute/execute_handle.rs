//! A running tool container, and everything a caller can do with it.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::Stream;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult, ListToolsResult,
    PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResult,
};

use super::super::channel_request;
use super::{Filetree, FiletreeStream, Read, ReadStream, Transfer, WritePath};
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::endpoints::containers::client::answered::{
    McpCallTool, McpListResources, McpListTools, McpNotifications, McpReadResource,
};
use crate::endpoints::containers::client::{ChannelStream, Decoded, OpenError, Scoped, UnaryError, WaitError};
use crate::endpoints::containers::tools::run::server;
use crate::shared::containers::{read, transfer, write_path};
use crate::shared::mcp;

/// The servers' notifications, for as long as the channel lives.
pub type McpNotificationsStream = ChannelStream<McpNotifications>;

/// The scope a run opened, held for the container's life.
///
/// Every channel a caller may open into a tool container is a method
/// here: the tree watched, a file read or written, the five MCP
/// exchanges into the server the container runs, and the stop. Each
/// opens its own channel, so several may be in flight at once. Clones
/// share the scope, and [`wait`](Self::wait) on any of them reports
/// the same end.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame. Ending a run is [`stop`](Self::stop),
/// or the connection going.
#[derive(Debug, Clone)]
pub struct ExecuteHandle(Arc<Scoped>);

/// One channel request of this family, as its bytes.
fn payload(frame: &channel_request::Frame) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes))?;
    Ok(bytes)
}

impl ExecuteHandle {
    pub(super) fn new(scoped: Arc<Scoped>) -> Self {
        ExecuteHandle(scoped)
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.0.scope()
    }

    /// The run's end: `Ok` when the scope finishes as it should — a
    /// stop, or the container's own end — and the provider's error
    /// when it ended that way. Resolves once and answers the same way
    /// again after.
    pub async fn wait(&self) -> Result<(), WaitError<server::response::FrameError>> {
        self.0
            .wait(|payload| {
                Ok(match server::response::Frame::decode(payload)? {
                    // Sent only as the first response, before the
                    // id; after it, neither is a fact of the run, and
                    // the wire forbids them.
                    server::response::Frame::Id(_) | server::response::Frame::VolumeHeld(_) => Decoded::Skip,
                    server::response::Frame::Error(error) => Decoded::Error(error),
                })
            })
            .await
    }

    /// Stop the container, and with it every connector's scope on it.
    /// Nothing answers on the channel this opens; what answers is the
    /// scope's own finish, on [`wait`](Self::wait).
    pub async fn stop(&self) -> Result<(), OpenError> {
        let payload = payload(&channel_request::Frame::Stop).map_err(OpenError::Request)?;
        self.0.bare(&payload).await.map_err(OpenError::Send)
    }

    /// Watch the container's tree: a snapshot, then every change.
    pub async fn filetree(&self) -> Result<FiletreeStream, OpenError> {
        let payload = payload(&channel_request::Frame::Filetree).map_err(OpenError::Request)?;
        self.0.open::<Filetree>(&payload).await.map_err(OpenError::Send)
    }

    /// Read one file out of the container, as path components from
    /// its root.
    pub async fn read(&self, path: Vec<String>) -> Result<ReadStream, OpenError> {
        let payload = payload(&channel_request::Frame::Read(read::request::Request { path }))
            .map_err(OpenError::Request)?;
        self.0.open::<Read>(&payload).await.map_err(OpenError::Send)
    }

    /// Write one file into the container, whole, from `content`'s
    /// pieces; resolves when it is at the path.
    pub async fn write<S, E>(&self, path: Vec<String>, content: S) -> Result<(), UnaryError<WritePath>>
    where
        S: Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: fmt::Display + Send + 'static,
    {
        self.0
            .write::<WritePath, S, E>(
                move |write_id| {
                    payload(&channel_request::Frame::Write(write_path::request::Request {
                        write_id,
                        path,
                    }))
                },
                content,
            )
            .await
    }

    /// Copy one file out of this container into the container under
    /// `id`, at `destination`, without the bytes passing through here;
    /// resolves when it is at the destination. The caller must be
    /// running, or connected to, both containers, or the provider
    /// refuses.
    pub async fn transfer(&self, path: Vec<String>, id: String, destination: Vec<String>) -> Result<(), UnaryError<Transfer>> {
        let payload = payload(&channel_request::Frame::Transfer(transfer::request::Request { path, id, destination }))
            .map_err(UnaryError::Request)?;
        self.0.unary::<Transfer>(&payload).await
    }

    /// `tools/list` against the server inside the container.
    pub async fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult, UnaryError<McpListTools>> {
        let payload = payload(&channel_request::Frame::McpListTools(mcp::list_tools::request::Request(params)))
            .map_err(UnaryError::Request)?;
        self.0.unary::<McpListTools>(&payload).await
    }

    /// `resources/list` against the server inside the container.
    pub async fn list_resources(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListResourcesResult, UnaryError<McpListResources>> {
        let payload = payload(&channel_request::Frame::McpListResources(
            mcp::list_resources::request::Request(params),
        ))
        .map_err(UnaryError::Request)?;
        self.0.unary::<McpListResources>(&payload).await
    }

    /// `tools/call` against the server inside the container.
    pub async fn call_tool(
        &self,
        params: CallToolRequestParams,
    ) -> Result<CallToolResult, UnaryError<McpCallTool>> {
        let payload = payload(&channel_request::Frame::McpCallTool(mcp::call_tool::request::Request(params)))
            .map_err(UnaryError::Request)?;
        self.0.unary::<McpCallTool>(&payload).await
    }

    /// `resources/read` against the server inside the container.
    pub async fn read_resource(
        &self,
        params: ReadResourceRequestParams,
    ) -> Result<ReadResourceResult, UnaryError<McpReadResource>> {
        let payload = payload(&channel_request::Frame::McpReadResource(
            mcp::read_resource::request::Request(params),
        ))
        .map_err(UnaryError::Request)?;
        self.0.unary::<McpReadResource>(&payload).await
    }

    /// Everything the server inside the container says on its own
    /// account, for as long as the channel lives.
    pub async fn notifications(&self) -> Result<McpNotificationsStream, OpenError> {
        let payload = payload(&channel_request::Frame::McpNotifications(
            mcp::notifications::request::Request,
        ))
        .map_err(OpenError::Request)?;
        self.0.open::<McpNotifications>(&payload).await.map_err(OpenError::Send)
    }
}
