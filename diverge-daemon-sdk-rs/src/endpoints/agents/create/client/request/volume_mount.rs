//! One volume made visible inside an agent's container.

use serde::{Deserialize, Serialize};

/// A volume, mounted into the agent's container.
///
/// Host side first, then the container side — source before
/// destination, the order a mount reads in everywhere else.
///
/// # Why the host side is a name and an offset
///
/// Because a host path is not something a caller is allowed to
/// state. [`host_name`](Self::host_name) is a volume's name as it
/// was published, and [`host_relative_path`](Self::host_relative_path)
/// descends from wherever that maps to — so a caller reaches a
/// subdirectory of something it was offered, and nothing else. A
/// name is resolved and then descended, never validated: a caller
/// cannot escape upward, because there is no component it can write
/// that means "up" — the offset is components, and `..` is a name,
/// not an instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct VolumeMount {
    /// Which volume, by the name it was published under.
    pub host_name: String,
    /// How far into that volume to start, as path components
    /// relative to it.
    ///
    /// Empty mounts the volume itself, which is the common case;
    /// anything else mounts a subdirectory of it.
    pub host_relative_path: Vec<String>,
    /// Where it appears inside the container, as path components from
    /// the container's root.
    ///
    /// Empty means the root itself, which is refused — the image's
    /// own filesystem is there.
    pub container_path: Vec<String>,
    /// Whether the container's changes to the volume outlive the
    /// container.
    ///
    /// `true`: every change the container makes is in the volume
    /// when the container ends. `false`: the volume is as it was
    /// before the run when the container ends. Either way the
    /// container sees the volume's content as of its start.
    ///
    /// Present, always: a request states it and nothing infers it.
    pub persist: bool,
}
