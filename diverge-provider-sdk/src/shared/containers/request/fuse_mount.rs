//! One file the caller serves live into the container.

use serde::{Deserialize, Serialize};

/// A file the caller keeps, mounted into the container over FUSE:
/// where, under what id, and whether the container may write it.
///
/// The provider MUST mount every one before the container starts —
/// the proxy inside the container does, at its start — as a
/// filesystem of exactly one regular file: the mount point is the
/// file itself, and the directory around it stays whatever the image
/// or another mount made it. The file can be read and, unless
/// [`readonly`](Self::readonly), overwritten in place; it cannot be
/// moved or deleted — the kernel refuses to rename or unlink a mount
/// point. Its bytes are fetched from the caller on every open and
/// stored back on every changed close, over the two exchanges in
/// [`fuse`](super::super::fuse), each carrying the id: the caller
/// serves the file from wherever it keeps it, and nothing in the
/// container or the provider copies it in or reads it back. It is for
/// the credential files vendor CLIs rewrite when they refresh a login.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FuseMount {
    /// Where the file appears inside the container, as path
    /// components from the container's root — the shape every path in
    /// this crate takes, as
    /// [`container_path`](super::IdentityMount::container_path) does
    /// for content.
    ///
    /// No component is empty, `.` or `..`, and the path is a FILE's:
    /// it names no directory. A path inside a directory another mount
    /// provides is allowed — a file over a directory a volume brought
    /// is the ordinary case — but a path equal to another mount's is
    /// not, and neither is the root.
    pub container_path: Vec<String>,
    /// The caller's own id for the file: opaque, minted by the caller
    /// when it named the mount, and echoed back on every read and
    /// write the provider sends for it. Two mounts on one request may
    /// not share an id.
    pub id: String,
    /// Whether the container may write the file. `true`: every open
    /// for writing, truncate and write inside the container fails,
    /// and the provider never sends a write for this id. `false`:
    /// overwritable in place, the whole file stored back on each
    /// changed close.
    #[serde(default)]
    pub readonly: bool,
}
