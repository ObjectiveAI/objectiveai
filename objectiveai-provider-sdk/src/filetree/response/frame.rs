//! What a filetree response frame carries.

use serde::{Deserialize, Serialize};

use super::Node;

/// One change on a filetree stream, discriminated by `type`.
///
/// [`Snapshot`](Frame::Snapshot) establishes the tree. Every other
/// variant names exactly one node and says what became of it: it
/// appeared ([`Inserted`](Frame::Inserted)), changed in place
/// ([`Modified`](Frame::Modified)), changed location
/// ([`Moved`](Frame::Moved)), or ceased to exist
/// ([`Removed`](Frame::Removed)).
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
/// makes replaying an already-applied frame harmless, and therefore
/// what makes at-least-once delivery safe.
///
/// [`Moved`](Frame::Moved) is the one variant that reads the tree
/// rather than overwriting part of it, so it is the one place replay
/// is not free: re-applying a move is harmless while its source path
/// stays empty, but not if something has since taken that path.
///
/// Every `path` in every variant is a component vector relative to the
/// filetree root — one meaning of "path" throughout, matching
/// [`Node::Symlink`]'s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    /// The whole tree: the root's entries, recursively.
    Snapshot {
        /// The root's entries. The root itself is not among them and
        /// is not described — see [`Root`](super::Root).
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
        /// The node's path, unchanged by this frame. The last element
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
    /// [`Removed`](Frame::Removed), since there is no destination
    /// inside the tree to name.
    Moved {
        /// Where the node was, before this frame.
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
