//! A container opened for one file operation.

use std::sync::Arc;

use crate::containers::{self, Opened};
use crate::daemon::Daemon;
use crate::store::agents::Agent;
use crate::store::tools::Tool;

/// The agent's container, started if it was not up, with the file use
/// held on it for as long as the [`Opened`] lives: the idle clock stops
/// it `idle_seconds` after the use and every other one are over, which
/// is what "stopped when it finishes" means under the idle rule.
pub async fn open_agent(daemon: &Arc<Daemon>, agent: &Agent) -> Result<Opened, String> {
    containers::agent(daemon, agent)
        .await
        .map(|run| {
            let using = run.file_use();
            Opened::Agent(run, using)
        })
        .map_err(|error| error.to_string())
}

/// The tool's container, started if it was not up, with the file use
/// held on it for as long as the [`Opened`] lives — a use, not work: it
/// makes the tool neither active nor busy: a transient
/// tool's idle clock stops it `idle_seconds` after the operation and
/// every other use are over. A connected tool is connected to the
/// same way, and every file operation on it then answers the handle's
/// error: its files are the other daemon's. A dependency tool is
/// opened by nobody: its run is taken as it is, as [`Opened::Tool`],
/// and started or stopped by nothing here; its file use is held on its
/// agent's run, with which it stops (see the dependency handlers).
pub async fn open_tool(daemon: &Arc<Daemon>, tool: &Tool) -> Result<Opened, String> {
    let run = containers::use_tool(daemon, tool).await.map_err(|error| error.to_string())?;
    let using = run.file_use();
    Ok(Opened::Tool(run, Some(using)))
}
