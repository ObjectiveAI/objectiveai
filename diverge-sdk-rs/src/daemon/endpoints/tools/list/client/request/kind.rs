//! Which of its four origins a tool has.

use serde::{Deserialize, Serialize};

/// Created by the daemon from a template, connected to a container
/// somebody else runs, deployed for an agent's run as a transient
/// dependency, or made and kept as a service: the `kind` the tool's
/// [`Origin`](crate::daemon::endpoints::tools::list::server::response::Origin)
/// is tagged with, as a value of its own to narrow a list by. Snake
/// case on the wire, as the tag is: `"created"`, `"connected"`,
/// `"dependency"`, `"service"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A tool the daemon made from a template and runs.
    Created,
    /// A tool somebody else runs, which the daemon joins.
    Connected,
    /// A tool the daemon deployed for an agent's run, from the
    /// transient dependency tool template the agent's program
    /// declared, running while the agent's container does.
    Dependency,
    /// A service the daemon made from the dependency tool template an
    /// agent's program declared, and keeps for every agent attached
    /// to it.
    Service,
}
