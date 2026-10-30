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
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::containers::client::answered::{
    FuseMount, Info, McpCallTool, McpListResources, McpListTools, McpNotifications, McpReadResource, Secrets,
};
use crate::provider::endpoints::containers::client::{ChannelStream, OpenError, Scoped, UnaryError, WaitError};
use crate::provider::endpoints::containers::tools::run::server;
use crate::shared::containers::fuse::Kind;
use crate::shared::containers::info::Document;
use crate::shared::containers::secrets::Values;
use crate::shared::containers::{fuse_mount, read, transfer, write_path};
use crate::shared::mcp;
use crate::shared::mcp::attestation::Caller;
use crate::shared::mcp::session::Sessioned;

/// The servers' notifications, for as long as the channel lives.
pub type McpNotificationsStream = ChannelStream<McpNotifications>;

/// The scope a run opened, held for the container's life.
///
/// Every channel a caller may open into a tool container is a method
/// here: the tree watched, a file read or written, the image's
/// DIVERGE.md, the five MCP exchanges into the server the container
/// runs, the secrets' feed, and the stop. What the proxy says of the
/// container's activity is not: it is the
/// [`ExecuteStream`](super::ExecuteStream) `execute` handed back
/// beside this. Each opens its own channel, so several may be in
/// flight at once. Clones share the scope, and [`ended`](Self::ended)
/// on any of them reports the same end.
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
    ///
    /// # It reads nothing
    ///
    /// The main stream is the container's activity, and the
    /// [`ExecuteStream`](super::ExecuteStream) `execute` handed back
    /// is its reader. This waits for that reader to reach the end, so
    /// that no word is lost to a caller who only wanted to know the
    /// run is over; a caller that wants the end and not the activity
    /// drains the stream.
    pub async fn ended(&self) -> Result<(), WaitError<server::response::FrameError>> {
        self.0.ended().await
    }

    /// Stop the container.
    /// Nothing answers on the channel this opens; what answers is the
    /// scope's own finish, on [`ended`](Self::ended).
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
    /// running both containers, or the provider refuses.
    pub async fn transfer(&self, path: Vec<String>, id: String, destination: Vec<String>) -> Result<(), UnaryError<Transfer>> {
        let payload = payload(&channel_request::Frame::Transfer(transfer::request::Request { path, id, destination }))
            .map_err(UnaryError::Request)?;
        self.0.unary::<Transfer>(&payload).await
    }

    /// What the arguments may be: the image's `/DIVERGE.md`, parsed —
    /// its name, its schema and its body — or `None` for an image that ships
    /// none.
    pub async fn info(&self) -> Result<Option<Document>, UnaryError<Info>> {
        let payload = payload(&channel_request::Frame::Info).map_err(UnaryError::Request)?;
        self.0.unary::<Info>(&payload).await
    }

    /// One more file or directory served into the container at
    /// `container_path`, under `id`, while it runs: the provider's one
    /// answer — made, or why not — once the proxy has answered, and
    /// before any ask of the id. The mount lives until the run ends.
    pub async fn fuse_mount(&self, container_path: Vec<String>, id: String, kind: Kind) -> Result<fuse_mount::response::Frame, UnaryError<FuseMount>> {
        let payload = payload(&channel_request::Frame::FuseMount(fuse_mount::request::Request { container_path, id, kind }))
            .map_err(UnaryError::Request)?;
        self.0.unary::<FuseMount>(&payload).await
    }

    /// `tools/list` against the server inside the container, on
    /// `session`'s MCP session.
    pub async fn list_tools(
        &self,
        session: &Caller,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult, UnaryError<McpListTools>> {
        let request = Sessioned { session: session.clone(), request: mcp::list_tools::request::Request(params) };
        let payload = payload(&channel_request::Frame::McpListTools(request))
            .map_err(UnaryError::Request)?;
        self.0.unary::<McpListTools>(&payload).await
    }

    /// `resources/list` against the server inside the container, on
    /// `session`'s MCP session.
    pub async fn list_resources(
        &self,
        session: &Caller,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListResourcesResult, UnaryError<McpListResources>> {
        let request = Sessioned { session: session.clone(), request: mcp::list_resources::request::Request(params) };
        let payload = payload(&channel_request::Frame::McpListResources(request))
        .map_err(UnaryError::Request)?;
        self.0.unary::<McpListResources>(&payload).await
    }

    /// `tools/call` against the server inside the container, on
    /// `session`'s MCP session.
    pub async fn call_tool(
        &self,
        session: &Caller,
        params: CallToolRequestParams,
    ) -> Result<CallToolResult, UnaryError<McpCallTool>> {
        let request = Sessioned { session: session.clone(), request: mcp::call_tool::request::Request(params) };
        let payload = payload(&channel_request::Frame::McpCallTool(request))
            .map_err(UnaryError::Request)?;
        self.0.unary::<McpCallTool>(&payload).await
    }

    /// `resources/read` against the server inside the container, on
    /// `session`'s MCP session.
    pub async fn read_resource(
        &self,
        session: &Caller,
        params: ReadResourceRequestParams,
    ) -> Result<ReadResourceResult, UnaryError<McpReadResource>> {
        let request = Sessioned { session: session.clone(), request: mcp::read_resource::request::Request(params) };
        let payload = payload(&channel_request::Frame::McpReadResource(request))
        .map_err(UnaryError::Request)?;
        self.0.unary::<McpReadResource>(&payload).await
    }

    /// Everything the server inside the container says on its own
    /// account on `session`'s MCP session, for as long as the channel
    /// lives.
    pub async fn notifications(&self, session: &Caller) -> Result<McpNotificationsStream, OpenError> {
        let request = Sessioned { session: session.clone(), request: mcp::notifications::request::Request };
        let payload = payload(&channel_request::Frame::McpNotifications(request))
        .map_err(OpenError::Request)?;
        self.0.open::<McpNotifications>(&payload).await.map_err(OpenError::Send)
    }

    /// Feed the program its secrets' `values`, every name it
    /// declared, at an activation: resolves once the program holds
    /// them, relayed through the provider to the proxy, which
    /// delivers no exchange to the program until it does. The first
    /// feed is not this: it answers the registration, before the id.
    /// A refusal is [`Refused`](UnaryError::Refused), the program's
    /// own words through the proxy, and is the activation's error.
    pub async fn secrets(&self, values: Values) -> Result<(), UnaryError<Secrets>> {
        let payload = payload(&channel_request::Frame::Secrets(values)).map_err(UnaryError::Request)?;
        self.0.unary::<Secrets>(&payload).await
    }
}
