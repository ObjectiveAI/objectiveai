//! What a server's response frame carries.

use super::AgenticLoopChunk;

/// The payload of a [`ServerFrame::Response`](crate::frame::server::ServerFrame::Response).
///
/// A server's response frames are always channel `0` — the answer to
/// the client's own request — so there is exactly one thing they can
/// be. The tunnels do not appear here: their bytes travel the other
/// direction as [`request::Frame`](crate::agentic_loop::server::request::Frame), and
/// what comes BACK on them is a client response, not a server one.
///
/// # A frame type is never serialized
///
/// This enum is not a wire shape and carries no serde derives. It is
/// the DISPATCH layer: it says which payload a response frame holds,
/// and that is settled by the scope the frame arrives in rather than
/// by anything in its bytes. What goes on the wire is the payload's
/// own JSON, with nothing wrapped around it.
///
/// Which is why the payloads carry the derives and this does not. A
/// frame type that could be serialized would be a frame type that had
/// started describing the wire twice.
///
/// # One variant
///
/// An enum anyway. The wire reserves a whole byte for
/// the frame type and the server's space is open above `3`, so a
/// second thing a server can stream on channel `0` is a matter of
/// adding a variant rather than changing a shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One chunk of the answer to the client's request.
    AgenticLoop(AgenticLoopChunk),
}
