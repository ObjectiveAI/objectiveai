//! A container opened for one file operation.

use std::sync::Arc;

use crate::containers::{self, Opened};
use crate::daemon::Daemon;
use crate::store::agents::Agent;
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

/// The tool's container, started if it was not up, with the operation
/// in flight into it for as long as the [`Opened`] lives: a transient
/// tool's idle clock stops it `idle_seconds` after the operation and
/// every other use are over. A connected tool is connected to the
/// same way, and every file operation on it then answers the handle's
/// error: its files are the other daemon's. A dependency tool is
/// opened by nobody: its run is taken as it is, as [`Opened::Tool`]
/// with nothing in flight, and started or stopped by nothing here.
pub async fn open_tool(daemon: &Arc<Daemon>, tool: &Tool) -> Result<Opened, String> {
    let run = containers::use_tool(daemon, tool).await.map_err(|error| error.to_string())?;
    let flying = Arc::new(run.inflight.enter());
    Ok(Opened::Tool(run, Some(flying)))
}
