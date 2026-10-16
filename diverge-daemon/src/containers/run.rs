//! What is live for one running container.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::execute::ExecuteHandle as ConnectHandle;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::ExecuteHandle as AgentHandle;
use diverge_sdk::provider::endpoints::containers::tools::run::client::execute::{ExecuteHandle as ToolContainerHandle, McpNotificationsStream};
use diverge_sdk::shared::containers::fuse::Kind;
use diverge_sdk::shared::containers::fuse_mount;
use diverge_sdk::shared::mcp::attest;
use diverge_sdk::shared::mcp::attestation;
use bytes::Bytes;
use futures_util::{Stream, StreamExt as _};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult, ListToolsResult, MetaObject, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResult,
};
use tokio::sync::{Mutex, watch};
use tokio::task::AbortHandle;

use super::fuse::Mounts;
use super::mcp::Served;
use super::{ConnectionId, Frames, Inflight, ToolKey};
use crate::content::Pieces;
use crate::store::dependency_tools::DependencyTool;
use crate::store::{AccountId, AgentId};

/// A running agent: the scope held on its provider, whether a loop
/// runs in it, what is in flight for it, when it was last used, what
/// it mounts, the tools it is served, and the messages in flight.
pub struct AgentRun {
    /// The record.
    pub id: AgentId,
    /// The agent as the daemon attests it under `_meta`, on every call
    /// it sends outward and every chunk it says: the caller of the
    /// agent kind — template, index, name — with the image it was
    /// made from.
    pub attestation: attestation::Caller,
    /// The agent as a sender and a maker.
    pub sender: Creator,
    /// The provider it runs on.
    pub provider: Identity,
    /// The container's id there.
    pub container: String,
    /// The run scope: every channel into the container, and its end.
    pub handle: AgentHandle,
    /// Whether a loop runs in it now, as the proxy's two words say.
    pub loop_active: watch::Sender<bool>,
    /// The MCP exchanges in flight for it: its own, and its
    /// dependencies'. The other thing that makes it active.
    pub inflight: Arc<Inflight>,
    /// When it was last used; what the idle clock runs from.
    pub touched: watch::Sender<Instant>,
    /// What it mounts, served by the daemon.
    pub mounts: Arc<Mounts>,
    /// Every volume its record names in its mounts: held while it
    /// runs.
    pub volumes: Vec<reference::Volume>,
    /// The tools it is served: attached ones and its dependencies.
    pub served: Arc<Mutex<Served>>,
    /// The messages enqueued and not yet landed, by key, with who
    /// sent each.
    pub messages: Mutex<HashMap<String, Creator>>,
    /// The tasks that are the run's: the pump and the idle clock.
    pub tasks: Mutex<Vec<AbortHandle>>,
    /// Taken for the length of one edit of the record while it runs:
    /// two editors adding mounts are served one after the other.
    pub edits: Mutex<()>,
}

impl AgentRun {
    /// The container was used: the idle clock starts over.
    pub fn touch(&self) {
        self.touched.send_replace(Instant::now());
    }

    /// Whether the agent is active now: a loop runs in it, or an MCP
    /// exchange is in flight for it or for one of its dependencies.
    /// Exactly those two, and nothing else.
    pub fn is_active(&self) -> bool {
        *self.loop_active.borrow() || self.inflight.is_busy()
    }
}

/// The scope held on a tool: its own container's run on a provider,
/// or the connect scope held on another daemon.
pub enum ToolHandle {
    /// A created tool's container, run by the daemon.
    Run(ToolContainerHandle),
    /// A connected tool, served by another daemon over the daemon
    /// connection: its MCP exchanges, and nothing else.
    Connected(ConnectHandle),
}

/// The one sentence every file operation on a connected tool answers
/// with: a connected tool's container is another daemon's, and its
/// files are there.
pub const CONNECTED_FILES: &str = "a connected tool is another daemon's: its files are there";

impl ToolHandle {
    /// The tool's MCP tools.
    pub async fn list_tools(&self, params: Option<PaginatedRequestParams>) -> Result<ListToolsResult, String> {
        match self {
            ToolHandle::Run(handle) => handle.list_tools(params).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(handle) => handle.list_tools(params).await.map_err(|error| error.to_string()),
        }
    }

    /// The tool's MCP resources.
    pub async fn list_resources(&self, params: Option<PaginatedRequestParams>) -> Result<ListResourcesResult, String> {
        match self {
            ToolHandle::Run(handle) => handle.list_resources(params).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(handle) => handle.list_resources(params).await.map_err(|error| error.to_string()),
        }
    }

    /// One call, by the name the tool itself knows.
    pub async fn call_tool(&self, params: CallToolRequestParams) -> Result<CallToolResult, String> {
        match self {
            ToolHandle::Run(handle) => handle.call_tool(params).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(handle) => handle.call_tool(params).await.map_err(|error| error.to_string()),
        }
    }

    /// One resource read.
    pub async fn read_resource(&self, params: ReadResourceRequestParams) -> Result<ReadResourceResult, String> {
        match self {
            ToolHandle::Run(handle) => handle.read_resource(params).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(handle) => handle.read_resource(params).await.map_err(|error| error.to_string()),
        }
    }

    /// The tool's notifications, for as long as the scope lives.
    pub async fn notifications(&self) -> Result<McpNotificationsStream, String> {
        match self {
            ToolHandle::Run(handle) => handle.notifications().await.map_err(|error| error.to_string()),
            ToolHandle::Connected(handle) => handle.notifications().await.map_err(|error| error.to_string()),
        }
    }

    /// One more file or directory the daemon serves, mounted into the
    /// running container at `container_path` under `id`: the
    /// provider's one answer, waited for. A connected tool's container
    /// is another daemon's: [`CONNECTED_FILES`].
    pub async fn fuse_mount(&self, container_path: Vec<String>, id: String, kind: Kind) -> Result<fuse_mount::response::Frame, String> {
        match self {
            ToolHandle::Run(handle) => handle.fuse_mount(container_path, id, kind).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// The container's tree, watched. A connected tool's is on the
    /// other daemon: [`CONNECTED_FILES`].
    pub async fn filetree(&self) -> Result<Frames, String> {
        match self {
            ToolHandle::Run(handle) => {
                let stream = handle.filetree().await.map_err(|error| error.to_string())?;
                Ok(Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))))
            }
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// One file read out of the container. A connected tool's is on
    /// the other daemon: [`CONNECTED_FILES`].
    pub async fn read(&self, path: Vec<String>) -> Result<Pieces, String> {
        match self {
            ToolHandle::Run(handle) => {
                let stream = handle.read(path).await.map_err(|error| error.to_string())?;
                Ok(Box::pin(stream.map(|piece| piece.map_err(|error| error.to_string()))))
            }
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// One file written into the container, replaced whole. A
    /// connected tool's is on the other daemon: [`CONNECTED_FILES`].
    pub async fn write<S, E>(&self, path: Vec<String>, content: S) -> Result<(), String>
    where
        S: Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: std::fmt::Display + Send + 'static,
    {
        match self {
            ToolHandle::Run(handle) => handle.write(path, content).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// One file copied into the container under `id` on the same
    /// provider. A connected tool's is on the other daemon:
    /// [`CONNECTED_FILES`].
    pub async fn transfer(&self, path: Vec<String>, id: String, destination: Vec<String>) -> Result<(), String> {
        match self {
            ToolHandle::Run(handle) => handle.transfer(path, id, destination).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// End it: the container stopped, or the connection to the other
    /// daemon's tool let go.
    pub async fn stop(&self) {
        let _ = match self {
            ToolHandle::Run(handle) => handle.stop().await,
            ToolHandle::Connected(handle) => handle.disconnect().await,
        };
    }

    /// Wait for its end, however it comes.
    pub async fn wait(&self) {
        match self {
            ToolHandle::Run(handle) => {
                let _ = handle.wait().await;
            }
            ToolHandle::Connected(handle) => {
                handle.wait().await;
            }
        }
    }
}

/// A running tool: the scope held on it, what holds it up — the
/// connect scopes on it and the exchanges in flight into it — and what
/// it mounts: a record's container, a connected tool's connection, or
/// a dependency tool's container, a transient one's for its agent's
/// run or one run of a service's.
pub struct ToolRun {
    /// The record, or the dependency tool's row.
    pub id: ToolKey,
    /// The tool as the daemon attests it under `_meta`, on everything
    /// its server answers: created, connected or dependency, each with
    /// the image and the template it was made from — a connected
    /// tool's as the daemon that runs it told on the connect.
    pub attestation: attestation::Tool,
    /// The tool as a sender and a maker.
    pub sender: Creator,
    /// The account it runs under, if any.
    pub account: Option<AccountId>,
    /// The provider it runs on, or the daemon connection to its daemon
    /// goes through.
    pub provider: Identity,
    /// The container's id, for a container the daemon runs.
    pub container: Option<String>,
    /// The scope.
    pub handle: ToolHandle,
    /// Whether the tool is a service: a tool on record whose template's
    /// mode is, or a dependency tool whose template is. A service is
    /// held by its attachments and its connect scopes and retired by
    /// its clock, never stopped for want of use.
    pub service: bool,
    /// The connect scopes other daemons hold on it now: each a real
    /// connection, holding the run up for the scope's life.
    pub connections: Mutex<HashSet<ConnectionId>>,
    /// Whether a connect scope holds it now: `true` while
    /// `connections` is not empty, kept in step under their lock. What
    /// the idle clock waits on, beside what is in flight.
    pub held: watch::Sender<bool>,
    /// The exchanges in flight into it — an agent's MCP ask, a connect
    /// scope's, a file operation — each a real use, holding the run up
    /// until it is answered, however it ends. What a detach mid-call
    /// leaves standing: the call is answered still.
    pub inflight: Arc<Inflight>,
    /// When it was last used.
    pub touched: watch::Sender<Instant>,
    /// What it mounts, served by the daemon.
    pub mounts: Arc<Mounts>,
    /// Every volume its record names in its mounts: held while it
    /// runs.
    pub volumes: Vec<reference::Volume>,
    /// Whether the run has ended: `true` once, when it has, which every
    /// connect scope on the tool waits for.
    pub ended: watch::Sender<bool>,
    /// Taken for the length of one edit of the record while it runs:
    /// two editors adding mounts are served one after the other.
    pub edits: Mutex<()>,
    /// The tasks that are the run's: the waiter, and a connected
    /// tool's idle clock.
    pub tasks: Mutex<Vec<AbortHandle>>,
    /// What it is as a dependency tool, when it is one: the row, as
    /// it was when this run started.
    pub dependency: Option<DependencyTool>,
}

impl ToolRun {
    /// The container was used.
    pub fn touch(&self) {
        self.touched.send_replace(Instant::now());
    }

    /// Whether anything holds the run up: a connect scope another
    /// daemon holds on it, or an exchange in flight into it. Exactly
    /// those two — an attachment holds nothing, and an agent merely
    /// mid-loop nothing — and a transient tool's idle clock ends the
    /// run `idle_seconds` after both last became false. Asked under
    /// the connections lock by whoever lets go of a scope.
    pub fn held_now(&self, connections: &HashSet<ConnectionId>) -> bool {
        !connections.is_empty() || self.inflight.is_busy()
    }

    /// Attest `meta` as this tool's: the daemon's own word on
    /// everything the tool answers — every result, resource, tool and
    /// notification — whichever daemon runs it. A connected tool's
    /// answers arrive stamped by the daemon that runs it, and that
    /// stamp is replaced, not trusted.
    pub fn attest(&self, meta: &mut MetaObject) {
        attest(meta, &self.attestation);
    }
}
