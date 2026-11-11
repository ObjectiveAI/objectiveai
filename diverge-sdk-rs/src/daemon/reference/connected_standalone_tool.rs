//! One connected tool of the caller's, named either of two ways.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::daemons::DaemonId;

use super::Tool;

/// One connected tool of the caller's — another daemon's standalone
/// tool, held here by a
/// [`connected_standalone_tools::create`](crate::daemon::endpoints::connected_standalone_tools::create)
/// — by its name, or by the daemon it is on, the tool as that daemon
/// names it, and its index. See [`reference`](super) for which names
/// what. Untagged JSON, one object either way; an object with members
/// of both variants does not decode. A created standalone tool is
/// none of these: see [`Tool`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum ConnectedStandaloneTool {
    /// By name: `{"name":…}`.
    Name {
        /// The tool's name, as its create gave it. A tool given none
        /// is not reached this way.
        name: String,
    },
    /// By what it is held from, and its index:
    /// `{"daemon":…,"tool":…,"index":…}`. The three name it once and
    /// for all, as its list item carries them.
    DaemonToolIndex {
        /// The daemon the tool is on, by the ID of this daemon's record
        /// of it, as the create named it: see
        /// [`daemons`](crate::daemon::endpoints::daemons).
        daemon: DaemonId,
        /// The tool, as that daemon names it and as the create named
        /// it: see [`Tool`]. Read by that daemon and compared by this
        /// one.
        tool: Box<Tool>,
        /// The tool's number among all connected tools of the caller's
        /// ever held from that daemon's tool, as its list item carries
        /// it.
        index: u64,
    },
}
