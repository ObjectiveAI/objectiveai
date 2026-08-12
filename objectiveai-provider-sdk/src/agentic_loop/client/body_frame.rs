//! What a client's body frame carries.

/// The payload of a [`ClientFrame::Body`](crate::frame::client::ClientFrame::Body).
///
/// A body frame's bytes mean different things on different channels,
/// and this is the set of things they can mean. Which one applies is
/// not carried in the frame: it was settled when the channel opened,
/// by the type of the server request that opened it. A reader knows
/// before the body arrives, so the body does not repeat it.
///
/// A client only ever sends bodies on channels the SERVER opened —
/// its own request rides in a request frame, not a body — so nothing
/// here answers on channel `0`.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientBodyFrame {
    // Pending, each awaiting its payload type:
    //   Mcp
    //   Postgres
}
