//! A resource served over FUSE into every agent a template makes.

use serde::{Deserialize, Serialize};

/// One [resource](crate::daemon::endpoints::resources) of the
/// caller's, mounted over FUSE at a container path in every agent
/// made from the template, read-only or ephemeral. Which of the two
/// lists of the [`Template`](super::Template) it is on says whether it
/// is a file or a directory mount, and the resource named is of that
/// kind: a file resource on the file list, a directory resource on
/// the directory list, the other way round the agent create's error.
///
/// The daemon serves the mount itself, from the bytes it holds, as
/// the caller's FUSE server on the agent's run scope: no provider
/// volume stands behind it. Read-only, every mutation inside the
/// container answers the read-only byte and the resource is the same
/// bytes for every agent. Ephemeral, the daemon keeps a copy-on-write
/// layer per agent for the agent's life — the agent starts from the
/// resource as it is, its changes land in the layer, and the layer
/// goes with the agent — capped at [`overlay_disk`](Self::overlay_disk)
/// bytes, a change past the cap answered with an error.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceMount {
    /// The resource, by its id — its hash, as an
    /// [`upload`](crate::daemon::endpoints::resources::upload)
    /// answered it. A resource the caller does not hold when an agent
    /// is made from the template is that create's error; the template
    /// itself is made whether or not the resource is held yet.
    pub resource: String,
    /// Read-only, or ephemeral: see [`MountMode`].
    pub mode: MountMode,
    /// The most bytes the agent's own layer may hold, for an
    /// ephemeral mount: present exactly when [`mode`](Self::mode) is
    /// `ephemeral`, absent otherwise, and a mount that has it or lacks
    /// it the other way round is the template create's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_disk: Option<u64>,
    /// Where the mount appears inside the container, as path
    /// components from the container's root. No component is empty,
    /// `.` or `..`. The path is not the root, is no other mount's of
    /// the template's or of the agent's own, and lies inside no other
    /// mount's; no other mount's lies inside it.
    pub container_path: Vec<String>,
}

/// What becomes of a change a container makes to a resource mount.
/// JSON `read_only` or `ephemeral`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountMode {
    /// No change is made: every mutation is refused.
    ReadOnly,
    /// Every change lands in a layer of the agent's own, and goes with
    /// the agent.
    Ephemeral,
}
