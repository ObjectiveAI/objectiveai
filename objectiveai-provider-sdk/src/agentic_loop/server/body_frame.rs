//! What a server's body frame carries.

use super::super::client::response::AgenticLoopChunk;

/// The payload of a [`ServerFrame::Body`](crate::frame::server::ServerFrame::Body).
///
/// A body frame's bytes mean different things on different channels,
/// and this is the set of things they can mean. Which one applies is
/// not carried in the frame: it was settled when the channel opened —
/// channel `0` by the client's request, any other by the type of the
/// server request that opened it. A reader knows before the body
/// arrives, so the body does not repeat it.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerBodyFrame {
    /// On channel `0`: one chunk of the answer to the client's
    /// request.
    AgenticLoop(AgenticLoopChunk),
    // Pending, each awaiting its payload type:
    //   Mcp
    //   Postgres
}
