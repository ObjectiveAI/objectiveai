//! What is live for one running container.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::client::Cancel;
use diverge_sdk::daemon::client::stream::StreamError;
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::tools::expose::client::execute::ExecuteStream as ExposeStream;
use diverge_sdk::daemon::endpoints::tools::expose::server::response::{self as expose, Exposed};
use diverge_sdk::daemon::key;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::ExecuteHandle as AgentHandle;
use diverge_sdk::provider::endpoints::containers::tools::connect::client::execute::ExecuteHandle as JoinedHandle;
use diverge_sdk::provider::endpoints::containers::tools::run::client::execute::{ConnectionsStream, ExecuteHandle as ToolContainerHandle, McpNotificationsStream};
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::request::Image;
use bytes::Bytes;
use futures_util::{Stream, StreamExt as _};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};
use tokio::sync::{Mutex, watch};
use tokio::task::AbortHandle;

use super::fuse::Mounts;
use super::mcp::Served;
use super::{DependencyId, Frames, Inflight, ToolKey, User};
use crate::content::Pieces;
use crate::store::{AccountId, AgentId};

/// A running agent: the scope held on its provider, whether a loop
/// runs in it, what is in flight for it, when it was last used, what
/// it mounts, the tools it is served, and the messages in flight.
pub struct AgentRun {
    /// The record.
    pub id: AgentId,
    /// The agent as the daemon attests it under `_meta`, on every call
    /// it sends outward and every chunk it says: template, index, name.
    pub key: key::Agent,
    /// The image it was made from, attested beside the key.
    pub image: Image,
    /// The agent as a sender and a maker.
    pub sender: Creator,
    /// The account it runs under, if any.
    pub account: Option<AccountId>,
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

/// The scope held on a tool: its own container's run, or the connect
/// scope joined to somebody else's.
pub enum ToolHandle {
    /// A created tool's container, run by the daemon.
    Run(ToolContainerHandle),
    /// A connected tool's container, joined.
    Joined(JoinedHandle),
}

impl ToolHandle {
    /// The tool's MCP tools.
    pub async fn list_tools(&self, params: Option<PaginatedRequestParams>) -> Result<ListToolsResult, String> {
        match self {
            ToolHandle::Run(handle) => handle.list_tools(params).await.map_err(|error| error.to_string()),
            ToolHandle::Joined(handle) => handle.list_tools(params).await.map_err(|error| error.to_string()),
        }
    }

    /// The tool's MCP resources.
    pub async fn list_resources(&self, params: Option<PaginatedRequestParams>) -> Result<ListResourcesResult, String> {
        match self {
            ToolHandle::Run(handle) => handle.list_resources(params).await.map_err(|error| error.to_string()),
            ToolHandle::Joined(handle) => handle.list_resources(params).await.map_err(|error| error.to_string()),
        }
    }

    /// One call, by the name the tool itself knows.
    pub async fn call_tool(&self, params: CallToolRequestParams) -> Result<CallToolResult, String> {
        match self {
            ToolHandle::Run(handle) => handle.call_tool(params).await.map_err(|error| error.to_string()),
            ToolHandle::Joined(handle) => handle.call_tool(params).await.map_err(|error| error.to_string()),
        }
    }

    /// One resource read.
    pub async fn read_resource(&self, params: ReadResourceRequestParams) -> Result<ReadResourceResult, String> {
        match self {
            ToolHandle::Run(handle) => handle.read_resource(params).await.map_err(|error| error.to_string()),
            ToolHandle::Joined(handle) => handle.read_resource(params).await.map_err(|error| error.to_string()),
        }
    }

    /// The tool's notifications, for as long as the scope lives.
    pub async fn notifications(&self) -> Result<McpNotificationsStream, String> {
        match self {
            ToolHandle::Run(handle) => handle.notifications().await.map_err(|error| error.to_string()),
            ToolHandle::Joined(handle) => handle.notifications().await.map_err(|error| error.to_string()),
        }
    }

    /// The container's tree, watched.
    pub async fn filetree(&self) -> Result<Frames, String> {
        Ok(match self {
            ToolHandle::Run(handle) => {
                let stream = handle.filetree().await.map_err(|error| error.to_string())?;
                Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string())))
            }
            ToolHandle::Joined(handle) => {
                let stream = handle.filetree().await.map_err(|error| error.to_string())?;
                Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string())))
            }
        })
    }

    /// One file read out of the container.
    pub async fn read(&self, path: Vec<String>) -> Result<Pieces, String> {
        Ok(match self {
            ToolHandle::Run(handle) => {
                let stream = handle.read(path).await.map_err(|error| error.to_string())?;
                Box::pin(stream.map(|piece| piece.map_err(|error| error.to_string())))
            }
            ToolHandle::Joined(handle) => {
                let stream = handle.read(path).await.map_err(|error| error.to_string())?;
                Box::pin(stream.map(|piece| piece.map_err(|error| error.to_string())))
            }
        })
    }

    /// One file written into the container, replaced whole.
    pub async fn write<S, E>(&self, path: Vec<String>, content: S) -> Result<(), String>
    where
        S: Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: std::fmt::Display + Send + 'static,
    {
        match self {
            ToolHandle::Run(handle) => handle.write(path, content).await.map_err(|error| error.to_string()),
            ToolHandle::Joined(handle) => handle.write(path, content).await.map_err(|error| error.to_string()),
        }
    }

    /// One file copied into the container under `id` on the same
    /// provider.
    pub async fn transfer(&self, path: Vec<String>, id: String, destination: Vec<String>) -> Result<(), String> {
        match self {
            ToolHandle::Run(handle) => handle.transfer(path, id, destination).await.map_err(|error| error.to_string()),
            ToolHandle::Joined(handle) => handle.transfer(path, id, destination).await.map_err(|error| error.to_string()),
        }
    }

    /// End it: the container stopped, or the connector left.
    pub async fn stop(&self) {
        let _ = match self {
            ToolHandle::Run(handle) => handle.stop().await,
            ToolHandle::Joined(handle) => handle.disconnect().await,
        };
    }

    /// The connectors coming and going on the run's main stream, for a
    /// container the daemon runs; none for a joined one, whose
    /// connectors are its runner's to count.
    pub fn connections(&self) -> Option<ConnectionsStream> {
        match self {
            ToolHandle::Run(handle) => Some(handle.connections()),
            ToolHandle::Joined(_) => None,
        }
    }

    /// Wait for its end, however it comes.
    pub async fn wait(&self) {
        match self {
            ToolHandle::Run(handle) => {
                let _ = handle.wait().await;
            }
            ToolHandle::Joined(handle) => {
                let _ = handle.wait().await;
            }
        }
    }
}

/// What a dependency tool is, beyond a tool run: whose it is, what
/// it was deployed from — by id, and whole — and when.
pub struct Dependency {
    /// Its number among the dependencies deployed since the start.
    pub id: DependencyId,
    /// The agent it was deployed for.
    pub agent: AgentId,
    /// That agent, once and for all, with its name: what names the
    /// dependency beside its template.
    pub agent_key: key::Agent,
    /// The template it was deployed from, by id: the hash of its
    /// canonical bytes, unique among the agent's dependencies.
    pub template: String,
    /// The template it was deployed from, whole.
    pub declared: Template,
    /// When it was deployed.
    pub started: DateTime<Utc>,
}

/// The expose scope a connected tool's run was joined through, held
/// on the other daemon for the join's life: the stream the exposure
/// came on, read to its end — which is the tool's run ending there —
/// and the cancel that lets the exposure go.
pub struct Exposure {
    /// The scope's stream, after the first frame.
    stream: Mutex<ExposeStream>,
    /// The cancel.
    cancel: Cancel,
}

impl Exposure {
    /// The scope as it was opened, nothing read yet.
    pub fn new(stream: ExposeStream, cancel: Cancel) -> Self {
        Exposure {
            stream: Mutex::new(stream),
            cancel,
        }
    }

    /// The first frame: the exposure, or why there is none, in words
    /// a start's error carries.
    pub async fn first(&mut self) -> Result<Exposed, String> {
        match self.stream.get_mut().next().await {
            Some(Ok(expose::Frame::Exposed(exposed))) => Ok(exposed),
            Some(Ok(expose::Frame::NotFound)) => Err("the daemon named has no such tool".to_string()),
            Some(Ok(expose::Frame::Forbidden)) => Err("the daemon named does not allow this daemon to expose the tool".to_string()),
            Some(Ok(expose::Frame::Error(error))) => Err(format!("the daemon named could not expose the tool: {}", error.0)),
            Some(Err(error)) => Err(stream_end(&error)),
            None => Err("the daemon named finished the expose before answering".to_string()),
        }
    }

    /// The scope's end, however it comes: the tool's run ending on the
    /// other daemon, the daemon connection going, or a frame that will
    /// not read.
    pub async fn ended(&self) {
        let mut stream = self.stream.lock().await;
        while let Some(item) = stream.next().await {
            if item.is_err() {
                break;
            }
        }
    }

    /// Let the exposure go: the other daemon finishes the scope, and a
    /// tool held by nothing else there stops. Nothing to say to a
    /// scope that has finished already.
    pub async fn cancel(&self) {
        let _ = self.cancel.cancel().await;
    }
}

/// Why an expose stream stopped short, in words.
fn stream_end<E: std::fmt::Display>(error: &StreamError<E>) -> String {
    format!("the expose on the daemon named ended: {error}")
}

/// A running tool: the scope held on it, which containers and
/// exposures use it, who is connected to it from outside, and what it
/// mounts — a record's container, a connected tool's joined, or a
/// dependency deployed for an agent.
pub struct ToolRun {
    /// The record, or the deployment.
    pub id: ToolKey,
    /// The tool as the daemon attests it under `_meta`, on everything
    /// its server answers.
    pub key: key::Tool,
    /// The image it was made from, attested beside the key; none for
    /// a connected tool, whose image the daemon never sees.
    pub image: Option<Image>,
    /// The tool as a sender and a maker.
    pub sender: Creator,
    /// The account it runs under, if any.
    pub account: Option<AccountId>,
    /// The provider it runs on, or is joined through.
    pub provider: Identity,
    /// The container's id, for a container the daemon runs.
    pub container: Option<String>,
    /// The scope.
    pub handle: ToolHandle,
    /// The containers using it now, and the exposures holding it. A
    /// dependency has one user for its life, its agent, and is stopped
    /// with it.
    pub users: Mutex<HashSet<User>>,
    /// The connectors attached to it from outside now, as the provider
    /// tells of them: the other thing that holds a record's run up.
    pub connectors: watch::Sender<usize>,
    /// When it was last used.
    pub touched: watch::Sender<Instant>,
    /// What it mounts, served by the daemon.
    pub mounts: Arc<Mounts>,
    /// Every volume its record names in its mounts: held while it
    /// runs.
    pub volumes: Vec<reference::Volume>,
    /// Whether the run has ended: `true` once, when it has, which every
    /// expose scope on the tool waits for.
    pub ended: watch::Sender<bool>,
    /// The tasks that are the run's: the waiter.
    pub tasks: Mutex<Vec<AbortHandle>>,
    /// What it is as a dependency, when it is one.
    pub dependency: Option<Dependency>,
    /// The expose scope it was joined through, for a connected tool:
    /// its end is the run's, and the run's end lets it go.
    pub exposure: Option<Exposure>,
}

impl ToolRun {
    /// The container was used.
    pub fn touch(&self) {
        self.touched.send_replace(Instant::now());
    }

    /// Whether anything holds the run up: a container of the daemon's
    /// uses it, an expose scope holds it, or a connector is attached
    /// from outside. Exactly those three; a record's run is stopped
    /// when none holds. Asked under the users lock by whoever lets go
    /// of any of them.
    pub fn is_held(&self, users: &HashSet<User>) -> bool {
        !users.is_empty() || *self.connectors.borrow() > 0
    }
}
