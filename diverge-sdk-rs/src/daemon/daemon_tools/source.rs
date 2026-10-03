//! Where a transfer may read from.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::tools;
use super::Within;

/// One of the three places a transfer reads: the container's own
/// filesystem, a tool container attached to it, or a resource.
/// JSON-tagged by `kind`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    /// The container's own filesystem.
    Own {
        /// Where in it, as prefixes of the path read, each a list of
        /// components from the root: a read passes when its path is
        /// at or under any one of them. Absent when empty, and then
        /// anywhere.
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
    /// A resource of the caller's, by id: a file resource whole, or
    /// a file of a directory resource by its path in it.
    Resources {
        /// Which resources: any, or only those named by id.
        resources: Within<Vec<String>>,
    },
}
