//! One volume of the agent's provider made visible inside its
//! container.

use serde::{Deserialize, Serialize};

/// A volume of the provider the agent is pinned to, mounted into the
/// agent's container. Named on the create's
/// [`Provider`](super::Provider), never on the create itself: a
/// volume is one provider's, and so is an agent that mounts one. Not
/// one of the daemon's own [volumes](crate::endpoints::volumes) —
/// those reach a container as a [`FuseMount`](super::FuseMount),
/// served across by the daemon.
///
/// Host side first, then the container side — source before
/// destination, the order a mount reads in everywhere else.
///
/// # Why the host side is a name and an offset
///
/// Because a host path is not something a caller is allowed to
/// state. [`volume_name`](Self::volume_name) is a volume's name as it
/// was published, and [`volume_relative_path`](Self::volume_relative_path)
/// descends from wherever that maps to — so a caller reaches a
/// subdirectory of something it was offered, and nothing else. A
/// name is resolved and then descended, never validated: a caller
/// cannot escape upward, because there is no component it can write
/// that means "up" — the offset is components, and `..` is a name,
/// not an instruction.
///
/// # Whether the changes stay is the volume's
///
/// Not here. Whether what the container writes into the volume is in
/// the volume when the container ends is the volume's own persist
/// mode, stated when the volume was made and changed only by an edit
/// of it — the same for every container that mounts it, and not a
/// mount's to say.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct VolumeMount {
    /// Which volume, by the name the provider's listing gives it.
    pub volume_name: String,
    /// How far into that volume to start, as path components
    /// relative to it.
    ///
    /// Empty mounts the volume itself, which is the common case;
    /// anything else mounts a subdirectory of it.
    pub volume_relative_path: Vec<String>,
    /// Where it appears inside the container, as path components from
    /// the container's root.
    ///
    /// Empty means the root itself, which is refused — the image's
    /// own filesystem is there.
    pub container_path: Vec<String>,
}
