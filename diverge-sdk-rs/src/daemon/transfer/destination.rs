//! The three places a transfer lands.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

/// Where a transfer puts what it copies: at a path in an agent's
/// container, at a path in a tool's, or at a path in a volume.
/// Untagged JSON, told apart by its members — `{"agent":…,"path":…}`,
/// `{"tool":…,"path":…}`, `{"volume":…,"path":…}` — and an object
/// with members of more than one does not decode.
///
/// # What lands where
///
/// A file source lands at `path`, replaced whole, as a
/// [`write`](crate::shared::containers::write_path) replaces it. A
/// directory source lands as `path`: each file of it at `path` joined
/// with the file's path within the source, every parent made, and
/// nothing already at the destination removed — a file there that the
/// source has no file for stays. A volume is written at rest, as the
/// provider's
/// [`volumes::write`](crate::provider::endpoints::volumes::write)
/// writes it: one a running container has, or another download, upload
/// or transfer is on, is the transfer's `Held`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Destination {
    /// A path in an agent's container.
    Agent {
        /// The agent: see [`reference::Agent`].
        agent: reference::Agent,
        /// The destination, as components from the container's root; at
        /// least one, each a name.
        path: Vec<String>,
    },
    /// A path in a tool's container.
    Tool {
        /// The tool: see [`reference::Tool`].
        tool: reference::Tool,
        /// The destination, as components from the container's root; at
        /// least one, each a name.
        path: Vec<String>,
    },
    /// A path in a volume.
    Volume {
        /// The volume: see [`reference::Volume`].
        volume: reference::Volume,
        /// The destination, as components from the volume's root; at
        /// least one, each a name.
        path: Vec<String>,
    },
}
