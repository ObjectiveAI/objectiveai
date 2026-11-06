//! What is live for one running container.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::execute::{End as ConnectEnd, ExecuteHandle as ConnectHandle};
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::ExecuteHandle as AgentHandle;
use diverge_sdk::provider::endpoints::containers::tools::run::client::execute::{ExecuteHandle as ToolContainerHandle, McpNotificationsStream};
use diverge_sdk::provider::endpoints::containers::client::{UnaryError, WaitError};
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
use super::lifecycle::{End, RunLog};
use super::mcp::{Context, Served};
use super::secrets::Secrets;
use super::{ConnectionId, Frames, Inflight, Key, ToolKey, Use};
use crate::content::Pieces;
use crate::daemon::Daemon;
use crate::store::dependency_tools::DependencyTool;
use crate::store::AgentId;

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
    /// The file uses in progress on it — a filetree watch, a download,
    /// an upload, a transfer — each holding the run up for its length,
    /// as `inflight` does, but no work: they make it neither active nor
    /// busy. What the idle clock waits on, beside `inflight`.
    pub files: Arc<Inflight>,
    /// When it was last used; what the idle clock runs from.
    pub touched: watch::Sender<Instant>,
    /// What it mounts, served by the daemon.
    pub mounts: Arc<Mounts>,
    /// Every volume its record names in its mounts: held while it
    /// runs.
    pub volumes: Vec<reference::Volume>,
    /// The tools it is served: attached ones and its dependencies.
    pub served: Arc<Mutex<Served>>,
    /// The messages enqueued and not yet swept, by key, with who sent
    /// each: what a user part is logged as. See [`Message`] for when
    /// one leaves.
    pub messages: Arc<Mutex<HashMap<String, Message>>>,
    /// The messages enqueued and not yet fated: entered before the run
    /// is activated for each, left at its fate. What an inactive asks
    /// before it activates the run again.
    pub pending: Arc<Inflight>,
    /// Its secrets, as its registration took them: the names its
    /// program takes, the keys they are bound to, and the slots held
    /// while it is active; `None` for a program that registered
    /// nothing. Under its lock for every activation and inactive.
    pub secrets: Mutex<Option<Secrets>>,
    /// The tasks that are the run's: the pump and the idle clock.
    pub tasks: Mutex<Vec<AbortHandle>>,
    /// Taken for the length of one edit of the record while it runs:
    /// two editors adding mounts are served one after the other.
    pub edits: Mutex<()>,
    /// The run's number, fresh from the daemon: what its log's
    /// lifecycle is kept under — see [`lifecycle`](super::lifecycle).
    pub epoch: u64,
    /// Its log, live, taken when it started: every lifecycle item of
    /// the run is kept through this one.
    pub log: Arc<RunLog>,
    /// How the run is ending, once whoever ends it first has said: what
    /// its `undeployed` carries. See [`End`].
    pub ending: Mutex<Option<End>>,
}

/// One message enqueued on an agent's run, as its `messages` keep it.
/// The rules, so the map holds only what can still be named:
///
/// 1. **Kept from the enqueue.** An entry is made when the message is
///    enqueued, before it goes out.
/// 2. **An undelivered one leaves at its fate.** A fate other than
///    delivered — cancelled, an error — removes the entry; so does the
///    fate's future dropped before it was known, by its guard.
/// 3. **A delivered one leaves two inactives on.** A delivered entry is
///    marked so; every user part of a message arrives inside the loop
///    that took it, before that loop's `inactive`, but the pump may
///    still be reading the loop before when the mark lands. So the
///    pump, at every `inactive`, removes each delivered entry it marked
///    at an earlier one, and marks the delivered rest: an entry leaves
///    at the latest one loop after the loop that took it.
#[derive(Debug, Clone)]
pub struct Message {
    /// Who sent it.
    pub sender: Creator,
    /// Whether the loop took it: rule 3.
    pub delivered: bool,
    /// Whether an `inactive` has passed since it was delivered: rule 3.
    pub swept: bool,
}

impl Message {
    /// A message just enqueued: rule 1.
    pub fn new(sender: Creator) -> Self {
        Message { sender, delivered: false, swept: false }
    }
}

impl AgentRun {
    /// Rule 3 of [`Message`], at an `inactive`: every delivered entry
    /// marked at an earlier one removed, and the delivered rest marked.
    pub async fn sweep_messages(&self) {
        let mut messages = self.messages.lock().await;
        messages.retain(|_, message| !(message.delivered && message.swept));
        for message in messages.values_mut() {
            if message.delivered {
                message.swept = true;
            }
        }
    }

    /// The container was used: the idle clock starts over.
    pub fn touch(&self) {
        self.touched.send_replace(Instant::now());
    }

    /// Say how the run is ending, unless someone has already: the first
    /// word stands.
    pub async fn end_as(&self, end: End) {
        let mut ending = self.ending.lock().await;
        if ending.is_none() {
            *ending = Some(end);
        }
    }

    /// How the run is ending, as said; an end nobody told is expected.
    pub async fn end(&self) -> End {
        self.ending.lock().await.unwrap_or(End::Expected)
    }

    /// One file use begins on the run (see `files`): it holds the run
    /// up until the last clone is dropped, which touches it.
    pub fn file_use(&self) -> Arc<Use> {
        Arc::new(self.files.enter_use(self.touched.clone()))
    }

    /// Whether the agent is active now: a loop runs in it, or an MCP
    /// exchange is in flight for it or for one of its dependencies.
    /// Exactly those two, and nothing else.
    pub fn is_active(&self) -> bool {
        *self.loop_active.borrow() || self.inflight.is_busy()
    }

    /// The merge's context for the agent: its tools, its caller and
    /// its sender — what its own container's exchanges are answered
    /// with, and what an exchange run as the agent is answered with.
    pub fn context(&self, daemon: &Arc<Daemon>) -> Context {
        Context {
            daemon: Arc::clone(daemon),
            user: Key::Agent(self.id),
            caller: Some(self.attestation.clone()),
            sender: self.sender.clone(),
            served: Arc::clone(&self.served),
        }
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
    /// The tool's MCP tools, on `session`'s MCP session. A connected
    /// tool's session is the far daemon's to name, by this daemon's
    /// identity: see [`session`](diverge_sdk::shared::mcp::session).
    pub async fn list_tools(&self, session: &attestation::Caller, params: Option<PaginatedRequestParams>) -> Result<ListToolsResult, AskError> {
        match self {
            ToolHandle::Run(handle) => handle.list_tools(session, params).await.map_err(AskError::of),
            ToolHandle::Connected(handle) => handle.list_tools(params).await.map_err(AskError::of),
        }
    }

    /// The tool's MCP resources, on `session`'s MCP session.
    pub async fn list_resources(&self, session: &attestation::Caller, params: Option<PaginatedRequestParams>) -> Result<ListResourcesResult, AskError> {
        match self {
            ToolHandle::Run(handle) => handle.list_resources(session, params).await.map_err(AskError::of),
            ToolHandle::Connected(handle) => handle.list_resources(params).await.map_err(AskError::of),
        }
    }

    /// One call, by the name the tool itself knows, on `session`'s
    /// MCP session.
    pub async fn call_tool(&self, session: &attestation::Caller, params: CallToolRequestParams) -> Result<CallToolResult, AskError> {
        match self {
            ToolHandle::Run(handle) => handle.call_tool(session, params).await.map_err(AskError::of),
            ToolHandle::Connected(handle) => handle.call_tool(params).await.map_err(AskError::of),
        }
    }

    /// One resource read, on `session`'s MCP session.
    pub async fn read_resource(&self, session: &attestation::Caller, params: ReadResourceRequestParams) -> Result<ReadResourceResult, AskError> {
        match self {
            ToolHandle::Run(handle) => handle.read_resource(session, params).await.map_err(AskError::of),
            ToolHandle::Connected(handle) => handle.read_resource(params).await.map_err(AskError::of),
        }
    }

    /// What the tool sends on `session`'s MCP session, for as long as
    /// the scope lives.
    pub async fn notifications(&self, session: &attestation::Caller) -> Result<McpNotificationsStream, String> {
        match self {
            ToolHandle::Run(handle) => handle.notifications(session).await.map_err(|error| error.to_string()),
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
                // The channel closing with no finish is the run's scope
                // ending — the container stopped — which ends a watch
                // as the run over, not as a failure.
                let stream = stream.take_while(|frame| std::future::ready(!matches!(frame, Err(diverge_sdk::provider::endpoints::containers::client::ChannelStreamError::Closed))));
                Ok(Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))))
            }
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// One command run in the container, its stream; as
    /// [`Opened::exec`](super::Opened::exec). A connected tool's runs on
    /// the other daemon: [`CONNECTED_FILES`].
    pub async fn exec(&self, command: diverge_sdk::shared::containers::exec::request::Request) -> Result<super::ExecFrames, String> {
        match self {
            ToolHandle::Run(handle) => {
                let stream = handle.exec(command).await.map_err(|error| error.to_string())?;
                let stream = stream.take_while(|frame| std::future::ready(!matches!(frame, Err(diverge_sdk::provider::endpoints::containers::client::ChannelStreamError::Closed))));
                Ok(Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))))
            }
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// A terminal started in the container; as
    /// [`Opened::terminal_open`](super::Opened::terminal_open). A
    /// connected tool's is on the other daemon: [`CONNECTED_FILES`].
    pub async fn terminal_open(&self, size: diverge_sdk::shared::containers::terminal::open::Open) -> Result<u32, String> {
        match self {
            ToolHandle::Run(handle) => handle.terminal_open(size).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// A held terminal's output; as
    /// [`Opened::terminal_attach`](super::Opened::terminal_attach).
    pub async fn terminal_attach(&self, id: u32) -> Result<super::TerminalFrames, String> {
        match self {
            ToolHandle::Run(handle) => {
                let stream = handle.terminal_attach(id).await.map_err(|error| error.to_string())?;
                let stream = stream.take_while(|frame| std::future::ready(!matches!(frame, Err(diverge_sdk::provider::endpoints::containers::client::ChannelStreamError::Closed))));
                Ok(Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))))
            }
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// Bytes written to a held terminal's stdin.
    pub async fn terminal_stdin(&self, id: u32, bytes: Vec<u8>) -> Result<(), String> {
        match self {
            ToolHandle::Run(handle) => handle.terminal_stdin(id, bytes).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// A held terminal's size set.
    pub async fn terminal_resize(&self, id: u32, size: diverge_sdk::shared::containers::terminal::open::Open) -> Result<(), String> {
        match self {
            ToolHandle::Run(handle) => handle.terminal_resize(id, size.cols, size.rows).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// A held terminal's shell killed, and the terminal forgotten.
    pub async fn terminal_close(&self, id: u32) -> Result<(), String> {
        match self {
            ToolHandle::Run(handle) => handle.terminal_close(id).await.map_err(|error| error.to_string()),
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// One HTTP request to the program's frontend; as
    /// [`Opened::frontend`](super::Opened::frontend). A connected tool's
    /// is on the other daemon: [`CONNECTED_FILES`].
    pub async fn frontend<S>(&self, head: diverge_sdk::shared::containers::frontend::request::Head, body: S) -> Result<(super::FrontendFrames, u32), String>
    where
        S: Stream<Item = Result<Bytes, std::convert::Infallible>> + Send + 'static,
    {
        match self {
            ToolHandle::Run(handle) => {
                let (stream, write_id) = handle.frontend(head, body).await.map_err(|error| error.to_string())?;
                let stream = stream.take_while(|frame| std::future::ready(!matches!(frame, Err(diverge_sdk::provider::endpoints::containers::client::ChannelStreamError::Closed))));
                Ok((Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))), write_id))
            }
            ToolHandle::Connected(_) => Err(CONNECTED_FILES.to_string()),
        }
    }

    /// Take back a frontend request's body, asked for or not.
    pub async fn forget(&self, write_id: u32) {
        if let ToolHandle::Run(handle) = self {
            handle.forget(write_id).await;
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

    /// Wait for its end, however it comes: how it ended, as an
    /// unexpected end would be told — the provider finishing it, the
    /// connection going, or its stream failing, and why.
    pub async fn wait(&self) -> Ended {
        match self {
            ToolHandle::Run(handle) => match handle.ended().await {
                Ok(()) => Ended::Finished,
                Err(WaitError::Closed) => Ended::Closed,
                Err(error) => Ended::Failed(error.to_string()),
            },
            ToolHandle::Connected(handle) => match handle.wait().await {
                ConnectEnd::Finished => Ended::Finished,
                ConnectEnd::Closed => Ended::Closed,
            },
        }
    }
}

/// How a run's scope ended, as the daemon heard it: what an end nobody
/// asked for is told as — see [`Ended::reason`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ended {
    /// The scope finished: the provider, or the far daemon, ended it.
    Finished,
    /// The connection the scope rode ended first.
    Closed,
    /// The scope's stream failed, in these words.
    Failed(String),
}

impl Ended {
    /// The reason an end nobody asked for carries: the connection
    /// gone is the provider lost, a finish the run exited, a failure
    /// failed.
    pub fn reason(&self) -> diverge_sdk::daemon::lifecycle::Reason {
        use diverge_sdk::daemon::lifecycle::Reason;
        match self {
            Ended::Finished => Reason::Exited,
            Ended::Closed => Reason::ProviderLost,
            Ended::Failed(_) => Reason::Failed,
        }
    }
}

/// Why an ask into a tool got no answer of the tool's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskError {
    /// The run's scope, or the connection under it, ended before the
    /// tool answered: the ask was interrupted, and the run's
    /// `undeployed` says why.
    Interrupted,
    /// Anything else, in the daemon's words.
    Other(String),
}

impl AskError {
    /// A unary exchange's failure, told apart: the channel closing or
    /// finishing with no answer is the run going down under it.
    fn of<A: diverge_sdk::provider::endpoints::containers::client::Answered>(error: UnaryError<A>) -> AskError
    where
        UnaryError<A>: std::fmt::Display,
    {
        match error {
            UnaryError::Closed | UnaryError::Unanswered => AskError::Interrupted,
            error => AskError::Other(error.to_string()),
        }
    }

    /// The MCP error a caller is answered with: an interrupted ask
    /// says so under `data`, with the run's reason when it is known.
    pub fn to_error_data(&self, reason: Option<diverge_sdk::daemon::lifecycle::Reason>) -> rmcp::model::ErrorData {
        match self {
            AskError::Interrupted => {
                let mut data = serde_json::json!({ "interrupted": true });
                if let Some(reason) = reason {
                    data["reason"] = serde_json::to_value(reason).unwrap_or(serde_json::Value::Null);
                }
                rmcp::model::ErrorData::internal_error("the tool's run ended before it answered", Some(data))
            }
            AskError::Other(error) => rmcp::model::ErrorData::internal_error(error.clone(), None),
        }
    }
}

impl std::fmt::Display for AskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AskError::Interrupted => f.write_str("the tool's run ended before it answered"),
            AskError::Other(error) => f.write_str(error),
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
    /// The file uses in progress on it — a filetree watch, a download,
    /// an upload, a transfer — each holding the run up for its length,
    /// as `inflight` does, but no work: they make it neither active nor
    /// busy. What the idle clock waits on, beside `inflight`.
    pub files: Arc<Inflight>,
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
    /// The tasks that are the run's: the pump — a connected tool's
    /// waiter — and a transient tool's idle clock.
    pub tasks: Mutex<Vec<AbortHandle>>,
    /// What it is as a dependency tool, when it is one: the row, as
    /// it was when this run started.
    pub dependency: Option<DependencyTool>,
    /// The callers' MCP sessions the tool has been asked on, by
    /// [`key`](diverge_sdk::shared::mcp::session::key), each with the
    /// reader that keeps its notifications on the tool's log from its
    /// first use, as [`tool_log::session`](crate::containers::tool_log::session)
    /// states; a session leaves with its connect scope, its agent, or
    /// the run.
    pub sessions: Mutex<HashMap<String, AbortHandle>>,
    /// Its secrets, as for an agent's run; a connected tool's `None`,
    /// since the daemon that runs it feeds its container.
    pub secrets: Mutex<Option<Secrets>>,
    /// The run's number, fresh from the daemon: what its log's
    /// lifecycle is kept under — see [`lifecycle`](super::lifecycle).
    pub epoch: u64,
    /// Its log, live, taken when it started: every lifecycle item of
    /// the run is kept through this one.
    pub log: Arc<RunLog>,
    /// How the run is ending, once whoever ends it first has said: what
    /// its `undeployed` carries. See [`End`].
    pub ending: Mutex<Option<End>>,
}

impl ToolRun {
    /// The container was used.
    pub fn touch(&self) {
        self.touched.send_replace(Instant::now());
    }

    /// Say how the run is ending, unless someone has already: the first
    /// word stands.
    pub async fn end_as(&self, end: End) {
        let mut ending = self.ending.lock().await;
        if ending.is_none() {
            *ending = Some(end);
        }
    }

    /// How the run is ending, as said; an end nobody told is expected.
    pub async fn end(&self) -> End {
        self.ending.lock().await.unwrap_or(End::Expected)
    }

    /// One file use begins on the run (see `files`): it holds the run
    /// up until the last clone is dropped, which touches it.
    pub fn file_use(&self) -> Arc<Use> {
        Arc::new(self.files.enter_use(self.touched.clone()))
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
