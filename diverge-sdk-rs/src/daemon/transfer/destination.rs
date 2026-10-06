//! The three places a transfer lands.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;
use super::Resource;

/// Where a transfer puts what it copies: at a path in an agent's
/// container, at a path in a tool's, or into a new resource. Untagged
/// JSON, told apart by its members — `{"agent":…,"path":…}`,
/// `{"tool":…,"path":…}`, `{"resource":{"description":…}}` — and an
/// object with members of more than one does not decode.
///
/// # What lands where
///
/// A file source lands at `path`, replaced whole, as a
/// [`write`](crate::shared::containers::write_path) replaces it. A
/// directory source lands as `path`: each file of it at `path` joined
/// with the file's path within the source, every parent made, and
/// nothing already at the destination removed — a file there that the
/// source has no file for stays. A resource destination makes a file
/// resource of a file and a directory resource of a directory, held by
/// its hash exactly as
/// [`resources::upload`](crate::daemon::endpoints::resources::upload)
/// would hold the same bytes, with the description given; a resource
/// held already is the same id, nothing new is kept, and the
/// description is the request's from then on.
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
    /// A new resource.
    Resource {
        /// The resource to make: see [`Resource`].
        resource: Resource,
    },
}
