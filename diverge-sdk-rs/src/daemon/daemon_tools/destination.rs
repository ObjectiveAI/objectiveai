//! Where a transfer may write to.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::tools;
use super::Within;

/// One of the three places a transfer writes: the container's own
/// filesystem, a tool container attached to it, or the resource
/// store. JSON-tagged by `kind`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Destination {
    /// The container's own filesystem.
    Own {
        /// Where in it, as prefixes of the path written, each a list
        /// of components from the root: a write passes when its path
        /// is at or under any one of them. Absent when empty, and
        /// then anywhere.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        paths: Vec<Vec<String>>,
    },
    /// A tool container attached to the container, by the same
    /// filter the tools list narrows by, read as a test — see
    /// [`daemon_tools`](super). A tool not attached is never within
    /// reach, whatever the filter.
    Tools {
        /// Which tools: any attached, or only those the filter
        /// passes.
        tools: Within<tools::list::client::request::Filter>,
        /// Where in each, as [`Own`](Self::Own) says. Absent when
        /// empty, and then anywhere.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        paths: Vec<Vec<String>>,
    },
    /// The resource store: a transfer here is an
    /// [`upload`](crate::daemon::endpoints::resources::upload), and
    /// makes a resource of what it carries — one file, or a directory
    /// of files — with the description the call gives, held by its
    /// hash and made BY the container. There is nothing to narrow: a
    /// resource that does not yet exist has no id to name.
    Resources,
}
