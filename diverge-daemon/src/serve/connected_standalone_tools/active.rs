//! Whether a connected tool is active, and whether its connect is held.

use crate::containers::ToolKey;
use crate::daemon::Daemon;
use crate::store::ConnectedToolId;

/// Whether the tool is deployed now: the daemon holds the other
/// daemon's connect on it. What a list reports as `deployed`, and what
/// a filter and a grant's `within` may ask about.
pub async fn deployed(daemon: &Daemon, id: ConnectedToolId) -> bool {
    daemon.live.tool_run(ToolKey::Connected(id)).await.is_some()
}

/// Whether the tool is active now: a call in flight. What a list
/// reports as `active`.
pub async fn active(daemon: &Daemon, id: ConnectedToolId) -> bool {
    daemon.live.is_busy(ToolKey::Connected(id)).await
}
