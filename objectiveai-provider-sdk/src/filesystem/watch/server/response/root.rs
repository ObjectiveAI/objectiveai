//! The tree a watch's frames build.

/// The materialized tree.
///
/// Not something a provider sends — something a caller keeps, and
/// folds each [`Frame`](super::Frame) into with
/// [`Root::update`](crate::filetree::response::Root::update). It is
/// here because it is the shape the stream adds up to, and a consumer
/// that has the frames needs the thing to put them in.
///
/// See [`filetree::response::Root`](crate::filetree::response::Root).
pub type Root = crate::filetree::response::Root;
