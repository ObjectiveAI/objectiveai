//! One volume of the agent's provider made visible inside its
//! container.

use diverge_provider_sdk::endpoints::volumes::Mode;
use serde::{Deserialize, Serialize};

/// A volume of the provider the agent is pinned to, mounted into the
/// agent's container. Named on the create's
/// [`Provider`](super::Provider), never on the create itself: a
/// volume is one provider's, and so is an agent that mounts one. A
/// volume of any other provider reaches the container as a
/// [`FuseMount`](super::FuseMount), served across by the daemon.
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
/// # What becomes of the changes is the volume's
///
/// Not here. Whether what the container writes into the volume is in
/// the volume when the container ends, discarded with the container,
/// or refused inside it is the volume's own [`Mode`], stated when the
/// volume was made and changed only by an edit of it — the same for
/// every container that mounts it, and not a mount's to say; a mount
/// only states which mode it means the volume to have. So is how
/// many may hold it: any number of containers and serves for an
/// ephemeral or a read-only volume, one at a time for a persistent
/// one.
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
    /// The mode the volume is meant to be in: `persistent`, it keeps
    /// what is written into it; `ephemeral`, every change is discarded
    /// afterwards, with the container or with the serve; `read_only`,
    /// nothing changes it. The mode is the volume's own, on its
    /// provider — its listing reports it,
    /// [`volumes::create`](diverge_provider_sdk::endpoints::volumes::create)
    /// states it and
    /// [`volumes::edit`](diverge_provider_sdk::endpoints::volumes::edit)
    /// changes it; see [`Mode`] — and this states which mode this
    /// mount means the volume to have. What the daemon does with a
    /// volume whose mode differs at the create, this revision does
    /// not state.
    pub mode: Mode,
    /// Where it appears inside the container, as path components from
    /// the container's root.
    ///
    /// Empty means the root itself, which is refused — the image's
    /// own filesystem is there.
    pub container_path: Vec<String>,
}
