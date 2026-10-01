//! A directory resource, or a subtree of one, served over FUSE into
//! every container a template makes.

use serde::{Deserialize, Serialize};

use super::ResourceMode;

/// One directory [resource](crate::daemon::endpoints::resources) of
/// the caller's, or a subtree of it, mounted over FUSE as a directory
/// at a container path in every agent or tool made from the template,
/// read-only or ephemeral. The resource named is a directory
/// resource; a file resource here is the agent's or tool's create's
/// error.
///
/// The daemon serves the mount itself, from the bytes it holds, as
/// the caller's FUSE server on the container's run scope: no provider
/// volume stands behind it. The whole tree under the subtree named is
/// what the container sees, and every entry beneath the mount point
/// can be created, overwritten, renamed and deleted as the mode
/// allows; the mount point alone is fixed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceDirectoryMount {
    /// The resource, by its id — its hash, as an
    /// [`upload`](crate::daemon::endpoints::resources::upload)
    /// answered it. A resource the caller does not hold when an agent
    /// or a tool is made from the template is that create's error; the template
    /// itself is made whether or not the resource is held yet.
    pub resource: String,
    /// Where in the resource, as path components from the resource's
    /// root; empty is the resource itself. Names a directory of the
    /// resource — one that some file's path passes through — and a
    /// path at which nothing is, or at which what is there is a file,
    /// is the agent's or tool's create's error. No component is empty, `.` or
    /// `..`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_relative_path: Vec<String>,
    /// Read-only, or ephemeral with its cap: see [`ResourceMode`].
    /// Flattened into this object's JSON.
    #[serde(flatten)]
    pub mode: ResourceMode,
    /// Where the directory appears inside the container, as path
    /// components from the container's root. No component is empty,
    /// `.` or `..`. The path is not the root, is no other mount's of
    /// the template's or of the agent's own, and lies inside no other
    /// mount's; no other mount's lies inside it.
    pub container_path: Vec<String>,
}
