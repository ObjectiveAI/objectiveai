//! One file, or one directory, the caller serves live into the
//! agent's container.

use serde::{Deserialize, Serialize};

/// A file or a directory the caller keeps, mounted into the agent's
/// container over FUSE: where, and under what id. Which of the two
/// it is, is which list of the [`Frame`](super::Frame) it is on.
///
/// Every one is mounted before the agent runs, as a filesystem the
/// caller serves: the mount point is the file or the directory
/// itself, made if absent with every missing parent directory made
/// too, and the directory around it stays whatever the image or
/// another mount made it. Nothing copies the contents in or reads
/// them back: every read, write, listing, removal, rename and new
/// directory is one ask back to the caller, carrying the id, and the
/// caller serves it from wherever it keeps the thing. It is for the
/// credential files vendor CLIs rewrite when they refresh a login.
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
/// Nothing here says a mount may not change. A caller that wants one
/// unchangeable refuses the mutation asks it receives, one at a
/// time, and the program in the container sees that operation fail.
/// Nothing is enforced on the caller's behalf.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FuseMount {
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
    /// The caller's own id for the mount: opaque, minted by the caller
    /// when it named the mount, and echoed back on every ask sent for
    /// it. Two mounts on one request, on either list, may not share
    /// an id.
    pub id: String,
}
