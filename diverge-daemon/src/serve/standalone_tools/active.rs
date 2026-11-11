//! Whether a tool on record is active, and its container's id while
//! it runs.

use crate::containers::ToolKey;
use crate::daemon::Daemon;
use crate::store::ToolId;

/// Whether the tool is deployed now: its container runs.
/// What a list reports as `deployed`, and what a filter and a grant's
/// `within` may ask about.
pub async fn deployed(daemon: &Daemon, id: ToolId) -> bool {
    daemon.live.tool_run(ToolKey::Record(id)).await.is_some()
}

/// Whether the tool is active now: working, a call or a read in
/// flight. What a list reports as `active`.
pub async fn active(daemon: &Daemon, id: ToolId) -> bool {
    daemon.live.is_busy(ToolKey::Record(id)).await
}

/// The id of the tool's container while it runs, which a list
/// reports.
pub async fn running(daemon: &Daemon, id: ToolId) -> Option<String> {
    daemon.live.tool_run(ToolKey::Record(id)).await.and_then(|run| run.container.clone())
}
