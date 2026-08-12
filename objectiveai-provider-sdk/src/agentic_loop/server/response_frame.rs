//! What a server's response frame carries.

use super::response::AgenticLoopChunk;

/// The payload of a [`ServerFrame::Response`](crate::frame::server::ServerFrame::Response).
///
/// A server's response frames are always channel `0` — the answer to
/// the client's own request — so there is exactly one thing they can
/// be. The tunnels do not appear here: their bytes travel the other
/// direction as [`ServerRequestFrame`](super::ServerRequestFrame), and
/// what comes BACK on them is a client response, not a server one.
///
/// One variant, and an enum anyway. The wire reserves a whole byte for
/// the frame type and the server's space is open above `3`, so a
/// second thing a server can stream on channel `0` is a matter of
/// adding a variant rather than changing a shape.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerResponseFrame {
    /// One chunk of the answer to the client's request.
    AgenticLoop(AgenticLoopChunk),
}
