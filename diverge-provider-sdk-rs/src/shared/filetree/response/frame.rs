//! What a filetree response frame carries.

use serde::{Deserialize, Serialize};

use super::Node;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// One change on a filetree stream.
///
/// [`Snapshot`](Frame::Snapshot) establishes the tree. Every other
/// variant names exactly one node and says what became of it: it
/// appeared ([`Inserted`](Frame::Inserted)), changed in place
/// ([`Modified`](Frame::Modified)), or ceased to exist
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
/// what makes at-least-once delivery safe: every variant overwrites
/// or clears one place in the tree, and none reads the tree first.
///
/// # A rename is two frames
///
/// There is no move. A node renamed — within one directory or across
/// the tree — is reported as what happened on disk:
/// [`Removed`](Frame::Removed) at the path it left and
/// [`Inserted`](Frame::Inserted) at the path it arrived at, the
/// inserted node complete, a directory with its whole subtree. The
/// pairing a filesystem offers for the two halves is not reliable
/// enough to promise a consumer, and the tree is right without it;
/// what a consumer loses is only the knowledge that the two were one
/// node.
///
/// Every `path` in every variant is a component vector relative to the
/// filetree root — one meaning of "path" throughout, matching
/// [`Node::Symlink`]'s.
///
/// # Variant ORDER is part of the wire format
///
/// Serialized in serde's default representation, which writes the
/// variant's INDEX rather than its name — the same arrangement
/// [`Node`] uses, and for the same reason: an index needs no lookahead
/// to read, which is what makes it encodable in a format with no
/// self-description.
///
/// It is also a constraint. Reordering these variants, or inserting
/// one among them, silently changes what existing bytes mean. New
/// variants go on the END; nowhere else is a compatible change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frame {
    /// The whole tree: the root's entries, recursively.
    ///
    /// The first frame of every stream, and the only frame that may
    /// come again: a source that lost track of the tree — a watch
    /// whose event queue overflowed — sends a fresh one rather than
    /// deltas it cannot know. Each replaces the tree whole.
    Snapshot {
        /// The root's entries. The root itself is not among them and
        /// is not described — see [`Root`](super::Root).
        children: Vec<Node>,
    },
    /// A node came into existence at a path that held nothing.
    ///
    /// Also how a node arrives by rename, from elsewhere in the tree
    /// or from outside it: from this path's point of view nothing was
    /// relocated, something appeared.
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
    /// A node ceased to exist. A directory takes its whole subtree with
    /// it — no per-descendant removals follow.
    ///
    /// Also how a node leaves by rename — to elsewhere in the tree,
    /// where an [`Inserted`](Frame::Inserted) reports its arrival, or
    /// out of it.
    Removed {
        /// The vanished node's path.
        path: Vec<String>,
    },
}

/// Postcard, where the rest of the crate is JSON.
///
/// A filetree stream is the one thing here that is both high-volume
/// and free to choose: it relays nothing, so no byte of it has to
/// survive a round trip unchanged, and nothing downstream reads it as
/// text. What it is instead is spammy — one frame per changed node,
/// indefinitely — so the envelope is worth minimizing.
///
/// Postcard drops field names entirely and varint-encodes every length
/// and integer, which puts a small delta within a couple of bytes of
/// the information it actually carries. `Removed` naming
/// `src/main.rs` is fourteen bytes, ten of them the two strings
/// themselves.
impl Encode for Frame {
    /// Postcard's own failure. It has few ways to happen when writing
    /// — a buffer that will not take bytes, mostly — since everything
    /// here is a shape it can always represent.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        postcard::to_io(self, &mut *out)?;
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Postcard's own failure.
    type Error = postcard::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        postcard::from_bytes(bytes)
    }
}
