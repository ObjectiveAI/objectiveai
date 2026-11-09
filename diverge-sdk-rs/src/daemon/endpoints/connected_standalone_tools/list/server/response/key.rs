//! A connected tool as a key: what never changes of it.

use crate::daemon::endpoints::daemons::DaemonId;
use crate::daemon::reference;

/// A [`ConnectedStandaloneTool`](super::ConnectedStandaloneTool) less
/// every member that may change while it is listed, and less its name
/// and its route: the daemon, the tool there and the index, which is
/// what a listing tells one connected tool from another by, and what a
/// [`key::Tool::ConnectedStandalone`](crate::daemon::key::Tool::ConnectedStandalone)
/// carries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    /// The daemon record's ID.
    pub daemon: DaemonId,
    /// The tool as that daemon refers to it.
    pub tool: reference::Tool,
    /// The index among the connected tools held from that tool.
    pub index: u64,
}
