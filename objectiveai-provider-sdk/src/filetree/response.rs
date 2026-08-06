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

use serde::{Deserialize, Serialize};

/// The filetree's root — the whole tree, as the root's entries.
///
/// The root is deliberately NOT a [`Node`]. A node is something the
/// tree contains, addressable by a path and subject to deltas; the
/// root is the thing doing the containing. It has no name, no
/// metadata, and no path — every path in this API is expressed
/// relative to it rather than including it, so there is no path that
/// names it and no delta that can be about it.
///
/// A newtype rather than a named field, so the root IS its entries:
/// serde treats it transparently and it goes on the wire as a bare
/// list, with no wrapper object naming a field that could only ever
/// hold one thing.
///
/// This is also the shape a consumer materializes: applying deltas
/// means editing this list, so what a consumer holds and what a
/// snapshot delivers are the same type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Root(
    /// The root's entries, recursively. An empty root is an empty list
    /// — never absent, for the same reason a directory's `children` is
    /// never absent.
    pub Vec<Node>,
);

impl Root {
    /// Fold one [`Event`] into this tree.
    ///
    /// This is THE fold. Every holder of a materialized tree applies
    /// events the same way, so that two consumers fed the same stream
    /// cannot disagree about what it meant.
    ///
    /// **Lenient by design.** An event that cannot be applied — an
    /// empty path, a parent that is missing or is not a directory — is
    /// DROPPED, never guessed at. The fold does not fabricate the
    /// directories a path implies: a tree that is missing a subtree is
    /// recoverable from the next snapshot, whereas a tree containing
    /// invented nodes is wrong in a way nothing detects. Nothing here
    /// panics, and no ordering is assumed.
    ///
    /// **Replay-safe, with one edge.** [`Inserted`](Event::Inserted)
    /// and [`Modified`](Event::Modified) both place a complete node,
    /// so applying either twice is a no-op, and applying one where the
    /// other was expected still lands the right value —
    /// deliberately, since that tolerance is what makes at-least-once
    /// delivery safe. [`Moved`](Event::Moved) is the exception: it
    /// reads the tree rather than overwriting part of it, so replaying
    /// one is harmless only while its source path stays empty.
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Snapshot { children } => self.snapshot(children),
            Event::Inserted { path, node } => self.inserted(path, node),
            Event::Modified { path, node } => self.modified(path, node),
            Event::Moved { path, new_path } => self.moved(path, new_path),
            Event::Removed { path } => self.removed(path),
        }
    }

    /// Replace the whole tree.
    fn snapshot(&mut self, children: Vec<Node>) {
        self.0 = children;
    }

    /// Place a node that did not exist.
    ///
    /// Identical to [`Self::modified`] on purpose. The wire
    /// distinction is information for the CONSUMER; the tree that
    /// results is the same either way, and keeping it that way is what
    /// makes the fold indifferent to replay.
    fn inserted(&mut self, path: Vec<String>, node: Node) {
        self.place(path, node);
    }

    /// Replace a node that already existed. See [`Self::inserted`].
    fn modified(&mut self, path: Vec<String>, node: Node) {
        self.place(path, node);
    }

    /// Relocate a node, renaming it to its new basename.
    ///
    /// The destination is validated BEFORE the node is detached. A
    /// move that cannot be completed must leave the tree alone rather
    /// than destroy the node on the way to a place it cannot reach.
    fn moved(&mut self, path: Vec<String>, new_path: Vec<String>) {
        let Some(name) = new_path.last().cloned() else {
            return;
        };
        if !self.has_parent(&new_path) {
            return;
        }
        let Some(mut node) = self.detach(&path) else {
            return;
        };
        // A move is the only event that changes a node's name, and the
        // new name is the only thing it changes. See `Event::Moved`.
        node.set_name(name);
        self.place(new_path, node);
    }

    /// Drop a node, and with it everything beneath it.
    fn removed(&mut self, path: Vec<String>) {
        self.detach(&path);
    }

    /// Insert or replace `node` at `path`. No-op if the path is empty
    /// or its parent chain is not present.
    fn place(&mut self, path: Vec<String>, node: Node) {
        let Some((leaf, parents)) = path.split_last() else {
            return;
        };
        let Some(siblings) = descend_mut(&mut self.0, parents) else {
            return;
        };
        match siblings.iter().position(|c| c.name() == leaf) {
            Some(i) => siblings[i] = node,
            None => siblings.push(node),
        }
    }

    /// Remove and return the node at `path`, if it is there.
    fn detach(&mut self, path: &[String]) -> Option<Node> {
        let (leaf, parents) = path.split_last()?;
        let siblings = descend_mut(&mut self.0, parents)?;
        let i = siblings.iter().position(|c| c.name() == leaf)?;
        Some(siblings.remove(i))
    }

    /// Whether the directory that would contain `path` exists.
    fn has_parent(&self, path: &[String]) -> bool {
        match path.split_last() {
            Some((_, parents)) => descend(&self.0, parents).is_some(),
            None => false,
        }
    }
}

/// Walk `comps` down from `children`, following each component into
/// its directory's entries. `None` if a component is absent or names
/// something that is not a directory.
fn descend<'a>(mut children: &'a [Node], comps: &[String]) -> Option<&'a [Node]> {
    for comp in comps {
        let i = children.iter().position(|c| c.name() == comp)?;
        children = children[i].children()?;
    }
    Some(children)
}

/// [`descend`], mutably.
fn descend_mut<'a>(
    mut children: &'a mut Vec<Node>,
    comps: &[String],
) -> Option<&'a mut Vec<Node>> {
    for comp in comps {
        let i = children.iter().position(|c| c.name() == comp)?;
        match children[i].children_mut() {
            Some(next) => children = next,
            None => return None,
        }
    }
    Some(children)
}

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
        /// its own — see [`Root`].
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
        /// `path` carried by [`Event::Inserted`], [`Event::Modified`],
        /// [`Event::Moved`] and [`Event::Removed`]. Every path in this
        /// API means the same thing, so a consumer walks a link's
        /// target down from the snapshot's child list exactly as it
        /// walks an event path, with no separate rule for links.
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

impl Node {
    /// This node's basename.
    fn name(&self) -> &str {
        match self {
            Node::File { name, .. }
            | Node::Directory { name, .. }
            | Node::Symlink { name, .. } => name,
        }
    }

    /// Rename this node. Only a move does this — see [`Event::Moved`].
    fn set_name(&mut self, new_name: String) {
        match self {
            Node::File { name, .. }
            | Node::Directory { name, .. }
            | Node::Symlink { name, .. } => *name = new_name,
        }
    }

    /// This node's entries — `None` for anything but a directory.
    fn children(&self) -> Option<&[Node]> {
        match self {
            Node::Directory { children, .. } => Some(children),
            _ => None,
        }
    }

    /// [`Self::children`], mutably.
    fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Node::Directory { children, .. } => Some(children),
            _ => None,
        }
    }
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
/// A delta that carries a node carries its COMPLETE value, never a
/// patch against a value the consumer is assumed to hold. That is what
/// makes replaying an already-applied event harmless, and therefore
/// what makes at-least-once delivery safe.
///
/// [`Moved`](Event::Moved) is the one variant that reads the tree
/// rather than overwriting part of it, so it is the one place replay
/// is not free: re-applying a move is harmless while its source path
/// stays empty, but not if something has since taken that path.
///
/// Every `path` in every variant is a component vector relative to the
/// filetree root — one meaning of "path" throughout, matching
/// [`Node::Symlink`]'s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// The whole tree: the root's entries, recursively.
    Snapshot {
        /// The root's entries. The root itself is not among them and
        /// is not described — see [`Root`].
        children: Vec<Node>,
    },
    /// A node came into existence at a path that held nothing.
    ///
    /// Also how a node that was moved in from OUTSIDE the filetree
    /// arrives: from this tree's point of view nothing was relocated,
    /// something simply appeared.
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
    /// Carries no node, because a relocation produces none. Renaming
    /// touches directory entries, not data, so the moved node's
    /// `modified_at`, `created_at` and `size` are all exactly what the
    /// consumer already holds. The only field a move can change is
    /// `name`, and that is the last component of `new_path`. A node
    /// payload here would be, for a directory, an entire re-transmitted
    /// subtree conveying nothing.
    ///
    /// It follows that a move never carries a modification. A rename
    /// ONTO an existing name — the atomic write-then-rename that
    /// editors and package managers perform — is a different inode
    /// taking over a name, and is reported as the two events it
    /// actually is.
    ///
    /// A node moved OUT of the filetree is not this — it is
    /// [`Removed`](Event::Removed), since there is no destination
    /// inside the tree to name.
    Moved {
        /// Where the node was, before this event.
        path: Vec<String>,
        /// Where the node is now. Its last component is the node's
        /// name, which a move that renames will have changed.
        new_path: Vec<String>,
    },
    /// A node ceased to exist. A directory takes its whole subtree with
    /// it — no per-descendant removals follow.
    ///
    /// Also how a node moved OUT of the filetree is reported.
    Removed {
        /// The vanished node's path.
        path: Vec<String>,
    },
}
