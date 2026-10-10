//! A container opened for one file operation, and closed after.

use std::sync::Arc;

use crate::containers::{self, Key, Opened, ToolKey, User};
use crate::daemon::Daemon;
use crate::store::agents::Agent;
use crate::serve::tools::Found;
use crate::store::tools::Tool;

/// The agent's container, started if it was not up; the idle clock
/// stops it after, which is what "stopped when it finishes" means
/// under the idle rule.
pub async fn open_agent(daemon: &Arc<Daemon>, agent: &Agent) -> Result<Opened, String> {
    containers::agent(daemon, agent)
        .await
        .map(Opened::Agent)
        .map_err(|error| error.to_string())
}

/// The tool's container, started for the operation as a user of its
/// own; [`close`] releases it, which stops it when nothing else uses
/// it. A connected tool is connected to the same way, and every file
/// operation on it then answers the handle's error: its files are the
/// other daemon's.
pub async fn open_tool(daemon: &Arc<Daemon>, tool: &Tool) -> Result<Opened, String> {
    containers::use_tool(daemon, tool, User::Container(Key::Tool(ToolKey::Record(tool.id))))
        .await
        .map(Opened::Tool)
        .map_err(|error| error.to_string())
}

/// What a tool reference found, opened: a record's container as
/// [`open_tool`] opens it; a dependency's run as it is, which runs
/// for its agent's life and is nobody's to start or stop.
pub async fn open_found(daemon: &Arc<Daemon>, found: &Found) -> Result<Opened, String> {
    match found {
        Found::Record(tool) => open_tool(daemon, tool).await,
        Found::Dependency(run) => Ok(Opened::Tool(Arc::clone(run))),
    }
}

/// The operation is over: a record's tool released, a dependency
/// left running.
pub async fn close(daemon: &Daemon, opened: &Opened) {
    if let Opened::Tool(run) = opened {
        containers::release(daemon, run.id, User::Container(Key::Tool(run.id))).await;
    }
}
