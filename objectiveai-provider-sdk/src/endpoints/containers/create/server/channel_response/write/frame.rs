//! What a server's response frame carries on a write channel.

/// Whether the file landed.
///
/// See [`write::response::Frame`](crate::shared::container::write::response::Frame).
pub type Frame = crate::shared::container::write::response::Frame;
