//! What is live for one running container.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::ExecuteHandle as AgentHandle;
use diverge_sdk::provider::endpoints::containers::tools::connect::client::execute::ExecuteHandle as JoinedHandle;
use diverge_sdk::provider::endpoints::containers::tools::run::client::execute::{ExecuteHandle as ToolContainerHandle, McpNotificationsStream};
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
use super::{Frames, Key};
use crate::content::Pieces;
use crate::store::{AccountId, AgentId, ToolId};

/// A running agent: the scope held on its provider, whether a loop
/// runs in it, when it was last used, what it mounts, the tools it is
/// served, and the messages in flight.
pub struct AgentRun {
    /// The record.
    pub id: AgentId,
    /// Its name as it was when started: the root of every chain of
    /// dependencies its run begins.
    pub name: Option<String>,
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

    /// Whether a loop runs in it now.
    pub fn is_active(&self) -> bool {
        *self.loop_active.borrow()
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

/// A running tool: the scope held on it, which containers use it,
/// and what it mounts and is served.
pub struct ToolRun {
    /// The record.
    pub id: ToolId,
    /// The tool as a sender and a maker.
    pub sender: Creator,
    /// The account it runs under, if any.
    pub account: Option<AccountId>,
    /// The provider it runs on, or is joined through.
    pub provider: Identity,
    /// The container's id, for a created tool.
    pub container: Option<String>,
    /// The scope.
    pub handle: ToolHandle,
    /// The containers using it now; the last to leave stops it.
    pub users: Mutex<HashSet<Key>>,
    /// When it was last used.
    pub touched: watch::Sender<Instant>,
    /// What it mounts, served by the daemon.
    pub mounts: Arc<Mounts>,
    /// Every volume its record names in its mounts: held while it
    /// runs.
    pub volumes: Vec<reference::Volume>,
    /// The tools it is served: its own dependencies.
    pub served: Arc<Mutex<Served>>,
    /// The tasks that are the run's: the waiter.
    pub tasks: Mutex<Vec<AbortHandle>>,
}

impl ToolRun {
    /// The container was used.
    pub fn touch(&self) {
        self.touched.send_replace(Instant::now());
    }
}
