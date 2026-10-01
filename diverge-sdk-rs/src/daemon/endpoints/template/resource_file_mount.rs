//! A file resource served over FUSE into every container a template
//! makes.

use serde::{Deserialize, Serialize};

use super::ResourceMode;

/// One file [resource](crate::daemon::endpoints::resources) of the
/// caller's, mounted over FUSE as one file at a container path in
/// every agent or tool made from the template, read-only or
/// ephemeral. The
/// resource named is a file resource; a directory resource here is
/// the agent's or tool's create's error.
///
/// The daemon serves the mount itself, from the bytes it holds, as
/// the caller's FUSE server on the container's run scope: no provider
/// volume stands behind it. The file is overwritten in place only,
/// as every FUSE file mount is; a program that replaces its file by
/// rename needs a [directory mount](super::ResourceDirectoryMount).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceFileMount {
    /// The resource, by its id — its hash, as an
    /// [`upload`](crate::daemon::endpoints::resources::upload)
    /// answered it. A resource the caller does not hold when an agent
    /// or a tool is made from the template is that create's error; the template
    /// itself is made whether or not the resource is held yet.
    pub resource: String,
    /// Read-only, or ephemeral with its cap: see [`ResourceMode`].
    /// Flattened into this object's JSON.
    #[serde(flatten)]
    pub mode: ResourceMode,
    /// Where the file appears inside the container, as path
    /// components from the container's root. No component is empty,
    /// `.` or `..`. The path is not the root, is no other mount's of
    /// the template's or of the agent's own, and lies inside no other
    /// mount's; no other mount's lies inside it.
    pub container_path: Vec<String>,
}
