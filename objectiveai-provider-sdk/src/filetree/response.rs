//! Filetree response data.
//!
//! A filetree response is a **stream**: one [`Event::Snapshot`]
//! carrying the whole tree, then one delta per change — inserted,
//! modified, moved, or removed — for as long as the caller watches.
//! There is no polling and no second full send: the snapshot
//! establishes the tree, and every later event names one node.
//!
//! The rules governing that sequence — snapshot first, exactly once,
//! deltas only after it — are ordering properties over a stream, which
//! no schema can express. They belong to the prose specification. What
//! this file defines is the vocabulary: what a node is, and what an
//! event is.

use schemars::JsonSchema;
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "filetree.Node")]
pub enum Node {
    /// A regular file.
    #[schemars(title = "File")]
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
    #[schemars(title = "Directory")]
    Directory {
        /// Basename of this directory. The watched root has no node of
        /// its own — it is represented by the snapshot's child list.
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
    #[schemars(title = "Symlink")]
    Symlink {
        /// Basename of this link.
        name: String,
        /// The link's target, as path components ALWAYS RELATIVE TO
        /// THE FILETREE ROOT — the same frame of reference as the
        /// `path` carried by [`Event::Upserted`] and
        /// [`Event::Removed`]. Every path in this API means the same
        /// thing, so a consumer walks a link's target down from the
        /// snapshot's child list exactly as it walks an event path,
        /// with no separate rule for links.
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

/// One event on a filetree stream, discriminated by `type`.
///
/// [`Snapshot`](Event::Snapshot) establishes the tree. Every other
/// variant names exactly one node and says what became of it: it
/// appeared ([`Inserted`](Event::Inserted)), changed in place
/// ([`Modified`](Event::Modified)), changed location
/// ([`Moved`](Event::Moved)), or ceased to exist
/// ([`Removed`](Event::Removed)).
///
/// Insertion and modification are distinguished because a consumer
/// usually wants to treat them differently — a newly appeared file is
/// not the same news as an existing one being written to. The
/// distinction is INFORMATION, not a constraint: a consumer with no
/// use for it may treat the two identically, and doing so is what
/// keeps the fold tolerant of replay and reordering.
///
/// Every delta carries the node's complete new value rather than a
/// patch against a value the consumer is assumed to hold. That is what
/// makes replaying an already-applied event harmless, and therefore
/// what makes at-least-once delivery safe.
///
/// Every `path` in every variant is a component vector relative to the
/// filetree root — one meaning of "path" throughout, matching
/// [`Node::Symlink`]'s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "filetree.Event")]
pub enum Event {
    /// The whole tree: the watched root's entries, recursively.
    #[schemars(title = "Snapshot")]
    Snapshot {
        /// The watched root's entries. The root's own identity is
        /// implicit — the caller asked for a path and knows what it
        /// asked for.
        children: Vec<Node>,
    },
    /// A node came into existence at a path that held nothing.
    ///
    /// Also how a node that was moved in from OUTSIDE the filetree
    /// arrives: from this tree's point of view nothing was relocated,
    /// something simply appeared.
    #[schemars(title = "Inserted")]
    Inserted {
        /// Where the node appeared. The last element equals `node`'s
        /// `name`.
        ///
        /// Components rather than a joined string: a path is a
        /// sequence, and joining it would invent a separator that then
        /// has to be escaped out of names that contain it.
        path: Vec<String>,
        /// The node's complete value. A directory carries its whole
        /// subtree.
        node: Node,
    },
    /// A node that already existed changed, staying where it was.
    #[schemars(title = "Modified")]
    Modified {
        /// The node's path, unchanged by this event. The last element
        /// equals `node`'s `name`.
        path: Vec<String>,
        /// The node's complete new value, replacing the old one. A
        /// directory carries its whole subtree, so this replaces rather
        /// than merges.
        node: Node,
    },
    /// A node changed location. Its identity is preserved: this is one
    /// node relocating, not one vanishing and another appearing.
    ///
    /// A node moved OUT of the filetree is not this — it is
    /// [`Removed`](Event::Removed), since there is no destination
    /// inside the tree to name.
    #[schemars(title = "Moved")]
    Moved {
        /// Where the node was, before this event.
        path: Vec<String>,
        /// Where the node is now. The last element equals `node`'s
        /// `name` — a move that renames changes the basename, and the
        /// node reflects that.
        new_path: Vec<String>,
        /// The node's complete value at its new location. A directory
        /// carries its whole subtree.
        node: Node,
    },
    /// A node ceased to exist. A directory takes its whole subtree with
    /// it — no per-descendant removals follow.
    ///
    /// Also how a node moved OUT of the filetree is reported.
    #[schemars(title = "Removed")]
    Removed {
        /// The vanished node's path.
        path: Vec<String>,
    },
}
