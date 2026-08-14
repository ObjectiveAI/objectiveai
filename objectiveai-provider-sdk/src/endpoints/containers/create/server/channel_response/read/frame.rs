//! What a server's channel response frame carries on a read channel.

/// A piece of the file, or the news that it moved while being read.
///
/// See [`read::response::Frame`](crate::shared::container::read::response::Frame).
pub type Frame<'a> = crate::shared::container::read::response::Frame<'a>;
