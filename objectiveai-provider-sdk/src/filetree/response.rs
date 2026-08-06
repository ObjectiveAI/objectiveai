//! Filetree response data.
//!
//! A filetree response is a **stream**: one [`Event::Snapshot`]
//! carrying the whole tree, then one [`Event::Upserted`] or
//! [`Event::Removed`] per change, for as long as the caller watches.
//! There is no polling and no second full send — the snapshot
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
        /// The link's target as path components, exactly as stored:
        /// never resolved, never followed. `.` and `..` appear as
        /// literal components — this is what the link says, not where
        /// it lands. `None` only when reading the link itself failed.
        ///
        /// Unlike the component vectors that [`Event::Upserted`] and
        /// [`Event::Removed`] call `path`, this one does not address a
        /// node within the tree. A link may point anywhere, including
        /// outside the watched root or at nothing at all, so these
        /// components are not resolvable against the snapshot.
        ///
        /// Absolute and relative targets are not distinguished: both
        /// arrive as a bare component list.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<Vec<String>>,
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
/// [`Snapshot`](Event::Snapshot) establishes the tree.
/// [`Upserted`](Event::Upserted) and [`Removed`](Event::Removed) each
/// name exactly one node by its path.
///
/// Both delta variants are **idempotent**: applying one twice leaves
/// the tree as it was after the first. That is what makes at-least-once
/// delivery safe, and it is why an upsert carries the node's full new
/// value rather than a patch against a value the consumer is assumed to
/// hold.
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
    /// A node came into existence, or its value changed.
    #[schemars(title = "Upserted")]
    Upserted {
        /// The node's full path relative to the watched root, as
        /// components. The last element equals `node`'s `name`.
        ///
        /// Components rather than a joined string: a path is a
        /// sequence, and joining it would invent a separator that then
        /// has to be escaped out of names that contain it.
        path: Vec<String>,
        /// The node's complete new value. A directory carries its whole
        /// subtree, so an upsert replaces rather than merges.
        node: Node,
    },
    /// A node ceased to exist. A directory takes its whole subtree with
    /// it — no per-descendant removals follow.
    #[schemars(title = "Removed")]
    Removed {
        /// The vanished node's full path relative to the watched root,
        /// as components.
        path: Vec<String>,
    },
}
