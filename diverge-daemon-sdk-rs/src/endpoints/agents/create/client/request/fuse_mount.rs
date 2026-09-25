//! One file, or one directory, of the daemon's host served live into
//! the agent's container.

use serde::{Deserialize, Serialize};

/// A file or a directory on the daemon's host, mounted into the
/// agent's container over FUSE: where it is on the host, and where
/// it appears in the container. Which of the two it is, is which
/// list of the [`Frame`](super::Frame) it is on.
///
/// Every one is mounted before the agent runs, as a filesystem the
/// daemon serves: the mount point is the file or the directory
/// itself, made if absent with every missing parent directory made
/// too, and the directory around it stays whatever the image or
/// another mount made it. Nothing copies the contents in or reads
/// them back: every read, write, listing, removal, rename and new
/// directory in the container is one ask to the daemon, which serves
/// it from the host path, live — so what the container sees is the
/// host's file as it is, and what it writes lands there. It is for
/// the credential files vendor CLIs rewrite when they refresh a
/// login.
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
/// mutation the host's filesystem allows, and the program in the
/// container sees the host's own refusals.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FuseMount {
    /// Where the file or the directory is on the daemon's host: an
    /// absolute path, as the host's own filesystem writes one —
    /// `/home/ada/.config/tool/credentials`,
    /// `C:\\Users\\ada\\.config\\tool` — and not components, since it
    /// is the host's path and the host's rules. A file mount names a
    /// regular file, a directory mount a directory, and either must
    /// exist when the agent is created; a path that does not, or is
    /// not of the kind its list says, is the create's error.
    pub daemon_path: String,
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
