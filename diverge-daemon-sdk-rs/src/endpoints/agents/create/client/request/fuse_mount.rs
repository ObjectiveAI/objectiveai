//! One file, or one directory, of a daemon volume served live into
//! the agent's container.

use serde::{Deserialize, Serialize};

/// A file or a directory in one of the daemon's own
/// [volumes](crate::endpoints::volumes), mounted into the agent's
/// container over FUSE: which volume, where in it, and where it
/// appears in the container. Which of the two it is, is which list of
/// the [`Frame`](super::Frame) it is on.
///
/// Every one is mounted before the agent runs, as a filesystem the
/// daemon serves: the mount point is the file or the directory
/// itself, made if absent with every missing parent directory made
/// too, and the directory around it stays whatever the image or
/// another mount made it. Nothing copies the contents in or reads
/// them back: every read, write, listing, removal, rename and new
/// directory in the container is one ask to the daemon, which serves
/// it from the volume, live — so what the container sees is the
/// volume as it is, and what it writes lands in it. It is for the
/// credential files vendor CLIs rewrite when they refresh a login.
///
/// # The volume is held for the agent's life
///
/// As a provider holds a volume a container mounts: from the create
/// until the agent is deleted, the volume is mounted, and nothing
/// examines, reads, writes, walks, resizes or deletes it through the
/// daemon's [`volumes`](crate::endpoints::volumes) meanwhile — each
/// of those is refused as mounted. Any number of the caller's agents
/// may mount one volume at once.
///
/// A FILE mount is one regular file that can be read and overwritten
/// in place — opened, truncated, written, closed — but never deleted
/// or moved, and never replaced by a rename: the mount point is the
/// file itself, and the kernel refuses to unlink or rename a mount
/// point, so a program that saves by writing a temporary beside the
/// file and renaming it over the file fails at the rename. A
/// DIRECTORY mount is for that program: a whole tree the caller
/// serves, whose every entry can be created, overwritten by either
/// method, renamed and deleted, and whose root alone is fixed.
///
/// Nothing here says a mount may not change: the daemon serves every
/// mutation the volume allows, and the program in the container sees
/// the volume's own refusals.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FuseMount {
    /// Which of the daemon's volumes, by the name its
    /// [listing](crate::endpoints::volumes::list) gives it. A name the
    /// daemon holds no volume by is the create's error.
    pub volume_name: String,
    /// Where in that volume, as path components from the volume's
    /// root; empty is the volume itself. A file mount names a regular
    /// file, a directory mount a directory, and either must exist
    /// when the agent is created; a path at which nothing is, or at
    /// which what is there is not of the kind its list says, is the
    /// create's error. No component is empty, `.` or `..`.
    pub volume_relative_path: Vec<String>,
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
