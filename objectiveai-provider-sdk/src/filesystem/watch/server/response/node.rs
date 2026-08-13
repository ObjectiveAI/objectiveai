//! One node of the watched tree.

/// A file, a directory or a symlink.
///
/// Carried by the frames that place a node, and held by
/// [`Root`](super::Root) once one is folded in. See
/// [`filetree::response::Node`](crate::filetree::response::Node).
pub type Node = crate::filetree::response::Node;
