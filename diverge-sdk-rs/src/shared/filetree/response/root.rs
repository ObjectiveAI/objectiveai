//! The filetree's root, and the fold that keeps it current.

use serde::{Deserialize, Serialize};

use super::{Frame, Node};

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
    /// Fold one [`Frame`] into this tree.
    ///
    /// This is THE fold. Every holder of a materialized tree applies
    /// frames the same way, so that two consumers fed the same stream
    /// cannot disagree about what it meant.
    ///
    /// **Lenient by design.** A frame that cannot be applied — an
    /// empty path, a parent that is missing or is not a directory — is
    /// DROPPED, never guessed at. The fold does not fabricate the
    /// directories a path implies: a tree that is missing a subtree is
    /// recoverable from the next snapshot, whereas a tree containing
    /// invented nodes is wrong in a way nothing detects. Nothing here
    /// panics, and no ordering is assumed.
    ///
    /// **Replay-safe.** [`Inserted`](Frame::Inserted) and
    /// [`Modified`](Frame::Modified) both place a complete node, so
    /// applying either twice is a no-op, and applying one where the
    /// other was expected still lands the right value —
    /// deliberately, since that tolerance is what makes at-least-once
    /// delivery safe. [`Removed`](Frame::Removed) of a path already
    /// empty is nothing, and a [`Snapshot`](Frame::Snapshot) replaces
    /// the tree whole whenever it comes. No frame reads the tree
    /// before changing it, so no order of arrival can leave it wrong.
    pub fn update(&mut self, frame: Frame) {
        match frame {
            Frame::Snapshot { children } => self.snapshot(children),
            Frame::Inserted { path, node } => self.inserted(path, node),
            Frame::Modified { path, node } => self.modified(path, node),
            Frame::Removed { path } => self.removed(path),
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
}

/// Walk `comps` down from `children`, following each component into
/// its directory's entries. `None` if a component is absent or names
/// something that is not a directory.
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
