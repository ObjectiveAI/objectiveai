//! One node of the filesystem tree.

use serde::{Deserialize, Serialize};

/// One node of the filesystem tree, discriminated by `type`.
///
/// A [`Directory`](Node::Directory) carries its children inline;
/// [`File`](Node::File) and [`Symlink`](Node::Symlink) are leaves. A
/// symlink is the link ITSELF and is never followed, so a dangling or
/// looping link is a leaf rather than an error or an infinite tree.
///
/// Every variant carries `name` — the basename, never a path — plus
/// `created_at` and `modified_at`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    /// A regular file.
    File {
        /// Basename of this file.
        name: String,
        /// Size in bytes. `None` when the stat could not be read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<u64>,
        /// Creation time (unix seconds), when the filesystem records a
        /// birth time. `None` where unsupported — this is display
        /// metadata and is never load-bearing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        created_at: Option<i64>,
        /// Last-modified time (unix seconds). `None` when the stat
        /// could not be read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modified_at: Option<i64>,
    },
    /// A directory, carrying its entries.
    Directory {
        /// Basename of this directory. The watched root has no node of
        /// its own — see [`Root`](super::Root).
        name: String,
        /// Creation time (unix seconds), when the filesystem records a
        /// birth time.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        created_at: Option<i64>,
        /// Last-modified time (unix seconds). A directory's mtime
        /// tracks entry add/remove, not changes within its children.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modified_at: Option<i64>,
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
        /// [`Frame::Modified`](super::Frame::Modified),
        /// [`Frame::Moved`](super::Frame::Moved) and
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        created_at: Option<i64>,
        /// Last-modified time (unix seconds).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modified_at: Option<i64>,
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

    /// Rename this node. Only a move does this — see
    /// [`Frame::Moved`](super::Frame::Moved).
    pub(super) fn set_name(&mut self, new_name: String) {
        match self {
            Node::File { name, .. }
            | Node::Directory { name, .. }
            | Node::Symlink { name, .. } => *name = new_name,
        }
    }

    /// This node's entries — `None` for anything but a directory.
    pub(super) fn children(&self) -> Option<&[Node]> {
        match self {
            Node::Directory { children, .. } => Some(children),
            _ => None,
        }
    }

    /// [`Self::children`], mutably.
    pub(super) fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Node::Directory { children, .. } => Some(children),
            _ => None,
        }
    }
}
