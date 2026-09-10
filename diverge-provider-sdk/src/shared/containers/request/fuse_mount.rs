//! One file, or one directory, the caller serves live into the
//! container.

use serde::{Deserialize, Serialize};

/// A file or a directory the caller keeps, mounted into the container
/// over FUSE: where, under what id, and whether the container may
/// change it. Which of the two it is, is which list of the
/// [`Container`](super::Container) it is on.
///
/// The provider MUST mount every one before the container starts —
/// the proxy inside the container does, at its start — as a
/// filesystem the caller serves: the mount point is the file or the
/// directory itself, made if absent with every missing parent
/// directory made too, and the directory around it stays whatever
/// the image or another mount made it. Nothing in the container or
/// the provider copies the contents in or reads them back: every
/// read, write, listing, removal, rename and new directory is one
/// exchange in [`fuse`](super::super::fuse), carrying the id, and the
/// caller serves it from wherever it keeps the thing. It is for the
/// credential files vendor CLIs rewrite when they refresh a login.
///
/// A FILE mount is one regular file that can be read and, unless
/// [`readonly`](Self::readonly), overwritten in place — opened,
/// truncated, written, closed — but never deleted or moved, and
/// never replaced by a rename: the mount point is the file itself,
/// and the kernel refuses to unlink or rename a mount point, so a
/// program that saves by writing a temporary beside the file and
/// renaming it over the file fails at the rename. A DIRECTORY mount
/// is for that program: a whole tree the caller serves, whose every
/// entry can be created, overwritten by either method, renamed and
/// deleted, and whose root alone is fixed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FuseMount {
    /// Where the mount appears inside the container, as path
    /// components from the container's root — the shape every path in
    /// this crate takes, as
    /// [`container_path`](super::IdentityMount::container_path) does
    /// for content.
    ///
    /// No component is empty, `.` or `..`. A path inside a directory
    /// another mount provides is allowed — a file over a directory a
    /// volume brought is the ordinary case — but a path equal to
    /// another mount's, or inside another FUSE directory mount, is
    /// not, and neither is the root.
    pub container_path: Vec<String>,
    /// The caller's own id for the mount: opaque, minted by the caller
    /// when it named the mount, and echoed back on every ask the
    /// provider sends for it. Two mounts on one request, on either
    /// list, may not share an id.
    pub id: String,
    /// Whether the container may change it. `true`: every open for
    /// writing, truncate, write, create, rename, removal and new
    /// directory inside the container fails, and the provider never
    /// sends a mutation for this id. `false`: the file is overwritable
    /// in place, or the tree fully editable, every change stored with
    /// the caller as it lands.
    #[serde(default)]
    pub readonly: bool,
}
