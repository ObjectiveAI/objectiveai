//! The tree and its frames as JSON, tagged.
//!
//! On the wire a filetree is postcard, which knows a variant by its
//! index and nothing else, so [`Frame`] and [`Node`] carry no tag of
//! their own. For a client that prints what it reads — the daemon's
//! filetree answers — the same data is written tagged: every object
//! carries `type`, snake case, beside its members —
//! `{"type":"snapshot","children":[…]}`,
//! `{"type":"file","name":…}`. [`frame`] and [`nodes`] are the
//! serializers an answer names for its filetree members; [`FrameJson`]
//! and [`NodeJson`] the shapes they write.

use serde::{Serialize, Serializer};

use super::{Frame, Node};

/// A frame as JSON: `type` is `snapshot`, `inserted`, `modified` or
/// `removed`, the members the frame's own, nodes as [`NodeJson`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameJson<'a> {
    /// The whole tree.
    Snapshot {
        /// The root's entries.
        children: Vec<NodeJson<'a>>,
    },
    /// A node appeared.
    Inserted {
        /// Where.
        path: &'a [String],
        /// The node, whole.
        node: NodeJson<'a>,
    },
    /// A node changed.
    Modified {
        /// Where.
        path: &'a [String],
        /// The node, whole and new.
        node: NodeJson<'a>,
    },
    /// A node vanished.
    Removed {
        /// Where it was.
        path: &'a [String],
    },
}

/// A node as JSON: `type` is `file`, `directory` or `symlink`, the
/// members the node's own, a directory's children as these.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeJson<'a> {
    /// A regular file.
    File {
        /// Its basename.
        name: &'a str,
        /// Its size in bytes, when known.
        size: Option<u64>,
        /// Its creation time, unix seconds, when known.
        created_at: Option<u64>,
        /// Its last-modified time, unix seconds, when known.
        modified_at: Option<u64>,
    },
    /// A directory, with its entries.
    Directory {
        /// Its basename.
        name: &'a str,
        /// Its creation time, unix seconds, when known.
        created_at: Option<u64>,
        /// Its last-modified time, unix seconds, when known.
        modified_at: Option<u64>,
        /// Whether changes beneath it stream.
        changes: bool,
        /// Its entries.
        children: Vec<NodeJson<'a>>,
    },
    /// A symbolic link, never followed.
    Symlink {
        /// Its basename.
        name: &'a str,
        /// Its target, as components from the filetree root.
        path: &'a [String],
        /// Its creation time, unix seconds, when known.
        created_at: Option<u64>,
        /// Its last-modified time, unix seconds, when known.
        modified_at: Option<u64>,
    },
}

impl Frame {
    /// The frame as JSON writes it.
    pub fn json(&self) -> FrameJson<'_> {
        match self {
            Frame::Snapshot { children } => FrameJson::Snapshot {
                children: children.iter().map(Node::json).collect(),
            },
            Frame::Inserted { path, node } => FrameJson::Inserted { path, node: node.json() },
            Frame::Modified { path, node } => FrameJson::Modified { path, node: node.json() },
            Frame::Removed { path } => FrameJson::Removed { path },
        }
    }
}

impl Node {
    /// The node as JSON writes it.
    pub fn json(&self) -> NodeJson<'_> {
        match self {
            Node::File {
                name,
                size,
                created_at,
                modified_at,
            } => NodeJson::File {
                name,
                size: *size,
                created_at: *created_at,
                modified_at: *modified_at,
            },
            Node::Directory {
                name,
                created_at,
                modified_at,
                changes,
                children,
            } => NodeJson::Directory {
                name,
                created_at: *created_at,
                modified_at: *modified_at,
                changes: *changes,
                children: children.iter().map(Node::json).collect(),
            },
            Node::Symlink {
                name,
                path,
                created_at,
                modified_at,
            } => NodeJson::Symlink {
                name,
                path,
                created_at: *created_at,
                modified_at: *modified_at,
            },
        }
    }
}

/// Serialize a frame as [`FrameJson`]: for `serialize_with`.
pub fn frame<S: Serializer>(frame: &Frame, serializer: S) -> Result<S::Ok, S::Error> {
    frame.json().serialize(serializer)
}

/// Serialize nodes as [`NodeJson`]s: for `serialize_with`.
pub fn nodes<S: Serializer>(nodes: &[Node], serializer: S) -> Result<S::Ok, S::Error> {
    nodes.iter().map(Node::json).collect::<Vec<_>>().serialize(serializer)
}
