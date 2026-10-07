//! Whether a tool is active, and its container's id while it runs.

use crate::daemon::Daemon;
use crate::store::ToolId;

/// Whether the tool is active now: for a created tool, its container
/// runs; for a connected one, the daemon holds a connect scope on it.
/// What a list reports, and what a filter and a grant's `within` may
/// ask about. No container runs yet.
pub fn active(_: &Daemon, _: ToolId) -> bool {
    false
}

/// The id of the created tool's container while it runs, which a
/// list reports on its origin. None runs yet.
pub fn running(_: &Daemon, _: ToolId) -> Option<String> {
    None
}
