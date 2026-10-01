//! What becomes of a change a container makes to a resource mount.

use serde::{Deserialize, Serialize};

/// Read-only, or ephemeral with a cap: the mode of a
/// [`ResourceFileMount`](super::ResourceFileMount) or a
/// [`ResourceDirectoryMount`](super::ResourceDirectoryMount), and
/// what the mode brings with it. An enum, so that an ephemeral
/// mount's cap exists exactly when the mode is ephemeral and a
/// read-only mount cannot carry one. Flattened into the mount's
/// JSON: `"mode":"read_only"`, or `"mode":"ephemeral"` beside
/// `"overlay_disk"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ResourceMode {
    /// No change is made: every mutation inside the container answers
    /// the read-only byte, and the resource is the same bytes for
    /// every container.
    ReadOnly,
    /// Every change lands in a layer of the container's own, kept by
    /// the daemon for the agent's or tool's life: the container
    /// starts from the resource as it is, its changes land in the
    /// layer, and the layer goes with the agent or tool.
    Ephemeral {
        /// The most bytes the container's layer may hold. A change that
        /// would take the layer past it is answered with an error, and
        /// the layer is as it was.
        overlay_disk: u64,
    },
}
