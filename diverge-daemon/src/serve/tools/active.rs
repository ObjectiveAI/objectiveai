//! Whether a tool is active, and its container's id while it runs.

use crate::daemon::Daemon;
use crate::store::ToolId;

/// Whether the tool is active now: for a created tool, its container
/// runs; for a connected one, the daemon holds a connect scope on it.
/// What a list reports, and what a filter and a grant's `within` may
/// ask about.
pub async fn active(daemon: &Daemon, id: ToolId) -> bool {
    daemon.live.tool_run(id).await.is_some()
}

/// The id of the created tool's container while it runs, which a
/// list reports on its origin.
pub async fn running(daemon: &Daemon, id: ToolId) -> Option<String> {
    daemon.live.tool_run(id).await.and_then(|run| run.container.clone())
}
