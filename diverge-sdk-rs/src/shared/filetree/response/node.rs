//! One node of the filesystem tree.

use serde::{Deserialize, Serialize};

/// One node of the filesystem tree.
///
/// A [`Directory`](Node::Directory) carries its children inline;
/// [`File`](Node::File) and [`Symlink`](Node::Symlink) are leaves. A
/// symlink is the link ITSELF and is never followed, so a dangling or
/// looping link is a leaf rather than an error or an infinite tree.
///
/// Every variant carries `name` — the basename, never a path — plus
/// `created_at` and `modified_at`. A directory alone carries
/// `changes`: whether what happens beneath it will be reported, which
/// is a question only a directory can answer, because only a
/// directory is watched — see the field.
///
/// # Times are unsigned seconds
///
/// The same representation
/// [`Volume::created`](crate::provider::endpoints::volumes::list::server::response::Volume::created)
/// uses, and unsigned for the same reason: nothing a provider offers
/// predates 1970, and a signed field's negative half would exist to
/// represent a state that never occurs. `Option` here is about
/// availability rather than sign — a filesystem that records no birth
/// time has nothing to report, which is not the same as reporting a
/// time before the epoch.
///
/// # Variant ORDER is part of the wire format
///
/// This enum is serialized in serde's default representation, which
/// writes the variant's INDEX rather than its name. That is what makes
/// it encodable at all in a format with no self-description — there is
/// no name to look up and no lookahead to do — and it is also a
/// constraint: reordering these variants, or inserting one among them,
/// silently changes what existing bytes mean.
///
/// New variants go on the END. Nowhere else is a compatible change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Node {
    /// A regular file.
    File {
        /// Basename of this file.
        name: String,
        /// Size in bytes. `None` when the stat could not be read.
        size: Option<u64>,
        /// Creation time (unix seconds), when the filesystem records a
        /// birth time. `None` where unsupported — this is display
        /// metadata and is never load-bearing.
        created_at: Option<u64>,
        /// Last-modified time (unix seconds). `None` when the stat
        /// could not be read.
        modified_at: Option<u64>,
    },
    /// A directory, carrying its entries.
    Directory {
        /// Basename of this directory. The watched root has no node of
        /// its own — see [`Root`](super::Root).
        name: String,
        /// Creation time (unix seconds), when the filesystem records a
        /// birth time.
        created_at: Option<u64>,
        /// Last-modified time (unix seconds). A directory's mtime
        /// tracks entry add/remove, not changes within its children.
        modified_at: Option<u64>,
        /// Whether changes beneath this directory stream.
        ///
        /// `true` is the ordinary case: what happens under it arrives
        /// as deltas. `false` is a directory the source could not
        /// watch — a watch limit reached, a corner it was refused —
        /// and with it everything beneath, files included: what is
        /// here is what the walk found, nothing under it will be
        /// reported until the next [`Snapshot`](super::Frame::Snapshot),
        /// and a consumer should treat it as possibly stale. Only a
        /// directory carries this, because only a directory is
        /// watched: a file's changes are its parent's to report, so a
        /// file cannot fail to be watched on its own.
        changes: bool,
        /// This directory's entries. An empty directory carries an
        /// empty list — this field is never absent, so a consumer never
        /// has to distinguish "no children" from "children unknown".
        children: Vec<Node>,
    },
    /// A symbolic link — the link itself, never its target.
    Symlink {
        /// Basename of this link.
        name: String,
        /// The link's target, as path components ALWAYS RELATIVE TO
        /// THE FILETREE ROOT — the same frame of reference as the
        /// `path` carried by [`Frame::Inserted`](super::Frame::Inserted),
        /// [`Frame::Modified`](super::Frame::Modified) and
        /// [`Frame::Removed`](super::Frame::Removed). Every path in
        /// this API means the same thing, so a consumer walks a link's
        /// target down from the snapshot's child list exactly as it
        /// walks a frame's path, with no separate rule for links.
        ///
        /// Addressable is not the same as resolved: the link is still
        /// never followed, and the components may name a node that
        /// does not exist — an ordinary dangling link.
        ///
        /// Always present. A link whose contents could not be read is
        /// not a symlink node with a missing target; it is a failure,
        /// and is reported as one. That is unrelated to dangling: a
        /// link pointing at nothing still reports its components
        /// perfectly well, because reading a link never touches its
        /// target.
        path: Vec<String>,
        /// Creation time (unix seconds), when the filesystem records a
        /// birth time.
        created_at: Option<u64>,
        /// Last-modified time (unix seconds).
        modified_at: Option<u64>,
    },
}

// Visible throughout `response` — but no wider. These exist so
// [`Root`](super::Root)'s fold can walk and edit a tree; they are not
// part of the specification and nothing outside this module should
// depend on them.
impl Node {
    /// This node's basename.
    pub(super) fn name(&self) -> &str {
        match self {
            Node::File { name, .. }
            | Node::Directory { name, .. }
            | Node::Symlink { name, .. } => name,
        }
    }

    /// This node's entries, mutably — `None` for anything but a
    /// directory.
    pub(super) fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Node::Directory { children, .. } => Some(children),
            _ => None,
        }
    }
}
