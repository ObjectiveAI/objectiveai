//! What a client's response frame carries on a write content channel.

/// A piece of the file being written.
///
/// See [`write::bytes::Frame`](crate::shared::container::write::bytes::Frame).
pub type Frame<'a> = crate::shared::container::write::bytes::Frame<'a>;
