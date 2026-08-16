//! What a client's response frame carries on a write content channel.

/// A piece of the file being written, or a failure to supply one.
///
/// See [`write_bytes::response::Frame`](crate::shared::container::write_bytes::response::Frame).
pub type Frame<'a> = crate::shared::container::write_bytes::response::Frame<'a>;
