//! Which of its two origins a tool has.

use serde::{Deserialize, Serialize};

/// Created by the daemon from a template, or connected to a
/// container somebody else runs: the `kind` the tool's
/// [`Origin`](crate::daemon::endpoints::tools::list::server::response::Origin)
/// is tagged with, as a value of its own to narrow a list by. Snake
/// case on the wire, as the tag is: `"created"`, `"connected"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A tool the daemon made from a template and runs.
    Created,
    /// A tool somebody else runs, which the daemon joins.
    Connected,
}
