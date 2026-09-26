//! One file, or one directory, of a provider's volume served live
//! into the agent's container, across the daemon.

use serde::{Deserialize, Serialize};

use crate::endpoints::agents::logs::server::response::Identity;

/// A file or a directory in a volume of some provider the daemon
/// knows, mounted into the agent's container over FUSE: which
/// provider, which of its volumes, where in it, and where it appears
/// in the container. Which of the two it is, is which list of the
/// [`Frame`](super::Frame) it is on.
///
/// # Any provider's volume, the daemon between
///
/// A volume is one provider's, and a container runs on one provider;
/// a FUSE mount is how a container reaches a volume that is not its
/// own provider's. The daemon stands between: it holds the volume
/// served on its provider — the provider protocol's `volumes::serve`,
/// which answers the nine asks a FUSE mount makes — and answers the
/// container's provider, which relays the mount's asks, by forwarding
/// each ask to that serve as it is and each answer back. Every mount
/// names its own provider, so one agent's mounts may come from
/// several; the daemon opens one serve per distinct provider and
/// volume for the agent's life. The provider named may be the one
/// the agent runs on. Nothing forbids it, though a
/// [`VolumeMount`](super::VolumeMount) on the create's
/// [`Provider`](super::Provider) reaches the same volume without the
/// round trips.
///
/// Every one is mounted before the agent runs, as a filesystem the
/// daemon serves: the mount point is the file or the directory
/// itself, made if absent with every missing parent directory made
/// too, and the directory around it stays whatever the image or
/// another mount made it. Nothing copies the contents in or reads
/// them back, and nothing is buffered anywhere along the way: every
/// stat, every read of a piece, every write of a piece, every
/// truncation, every change of mode, owner or times, every listing,
/// removal, rename and new directory in the container is one ask,
/// answered from the volume in place as it comes — so what the
/// container sees is the volume as it is, with the mode, owner and
/// times the volume records, and what it writes lands in it as it is
/// written. It is for the credential files vendor CLIs rewrite when
/// they refresh a login, kept in a volume of a provider that runs
/// beside the daemon.
///
/// # A volume that keeps nothing
///
/// A volume whose persist mode is `false` is served read-only to the
/// container: every read, listing and stat goes through, and every
/// write, truncation, change of attributes, removal, rename and new
/// directory is refused by the volume's provider, the program seeing
/// a read-only filesystem.
///
/// # The volume is held for the agent's life
///
/// As its provider holds a volume a container mounts: from the create
/// until the agent is deleted, the volume is served, and its provider
/// refuses to examine, read, write, walk, resize or delete it
/// meanwhile. Any number of the caller's agents may mount one volume
/// at once, beside any container of that provider that mounts it. A
/// serve the provider refuses at the create — no such volume, or one
/// held to itself — is the create's error.
///
/// A FILE mount is one regular file that can be read and overwritten
/// in place — opened, truncated, written, closed — but never deleted
/// or moved, and never replaced by a rename: the mount point is the
/// file itself, and the kernel refuses to unlink or rename a mount
/// point, so a program that saves by writing a temporary beside the
/// file and renaming it over the file fails at the rename. A
/// DIRECTORY mount is for that program: a whole tree, whose every
/// entry can be created, overwritten by either method, renamed and
/// deleted, and whose root alone is fixed.
///
/// Nothing here says a mount may not change: the daemon forwards
/// every mutation, the volume's provider allows what its volume
/// allows, and the program in the container sees that provider's own
/// refusals.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FuseMount {
    /// Which provider holds the volume: the same object the create's
    /// [`Provider`](super::Provider) names one by, and an agent's log
    /// names one by — `kind: "outgoing"` and the address the daemon
    /// dials, or `kind: "incoming_unbrokered"` and the identity the
    /// daemon's judging of the provider's credential answered. A
    /// provider the daemon does not know by that identity is the
    /// create's error.
    pub provider: Identity,
    /// Which of that provider's volumes, by the name its listing gives
    /// it, as the daemon lists that provider's volumes. A name the
    /// provider holds no volume by is the create's error.
    pub volume_name: String,
    /// Where in that volume, as path components from the volume's
    /// root; empty is the volume itself. A file mount names a regular
    /// file, a directory mount a directory, and either must exist
    /// when the agent is created; a path at which nothing is, or at
    /// which what is there is not of the kind its list says, is the
    /// create's error. No component is empty, `.` or `..`.
    pub volume_relative_path: Vec<String>,
    /// The persist mode the volume is meant to be in: `true`, it
    /// keeps what is written into it; `false`, it keeps nothing, and
    /// what a container writes is gone with the container. The mode
    /// is the volume's own, on its provider — its listing reports it,
    /// [`volumes::create`](diverge_provider_sdk::endpoints::volumes::create)
    /// states it and
    /// [`volumes::edit`](diverge_provider_sdk::endpoints::volumes::edit)
    /// changes it — and this states which mode this mount means the
    /// volume to have. What the daemon does with a volume whose mode
    /// differs at the create, this revision does not state.
    pub persist: bool,
    /// Where the mount appears inside the container, as path
    /// components from the container's root, as
    /// [`container_path`](super::VolumeMount::container_path) is for
    /// a volume on the create's provider.
    ///
    /// No component is empty, `.` or `..`. The path is not the root,
    /// is no other mount's, and lies inside no other mount's — a
    /// volume's or a FUSE one's — and no other mount's lies inside
    /// it.
    pub container_path: Vec<String>,
}
