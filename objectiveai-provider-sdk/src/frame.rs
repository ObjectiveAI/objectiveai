//! The wire frame.
//!
//! Every message is one WebSocket BINARY frame:
//!
//! ```text
//! [scope: varint][channel: varint][type: u8][payload…]
//! ```
//!
//! No length prefix — WebSocket already delimits messages, so carrying
//! one would be paying twice for the same fact.

/// One frame. The payload borrows from the buffer it came out of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// The client request this belongs to, and everything that happens
    /// because of it. Minted by the server; `0` means none yet, since
    /// a client request is sent before the scope exists.
    pub scope: u64,
    /// One sub-conversation within the scope. Only the server opens
    /// channels, so the two ends cannot collide.
    pub channel: u64,
    /// What this frame is, as the carried protocol defines it.
    /// Uninterpreted here.
    pub r#type: u8,
    /// The bytes, if this type carries any.
    pub payload: &'a [u8],
}
