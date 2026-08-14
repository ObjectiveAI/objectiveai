//! One change on the container's filesystem.

/// One change on the container's filesystem — the snapshot that
/// establishes it, or one node inserted, modified, moved or removed.
///
/// See [`filetree::response::Frame`](crate::filetree::response::Frame)
/// for the variants and for what makes the stream replay-safe.
pub type Frame = crate::filetree::response::Frame;
