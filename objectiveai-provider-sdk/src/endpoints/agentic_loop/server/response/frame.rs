//! What a server's response frame carries.

use super::AgenticLoopChunk;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The payload of a [`ServerFrame::Response`](crate::frame::server::ServerFrame::Response).
///
/// A server's response frames are always channel `0` — the answer to
/// the client's own request — so there is exactly one thing they can
/// be. The tunnels do not appear here: their bytes travel the other
/// direction as [`request::Frame`](crate::endpoints::agentic_loop::server::channel_request::Frame),
/// and what comes BACK on them is a client response, not a server one.
///
/// # A frame type is never serialized
///
/// This type is not a wire shape and carries no serde derives. It is
/// the DISPATCH layer: it says which payload a response frame holds,
/// and that is settled by the scope the frame arrives in rather than
/// by anything in its bytes. What goes on the wire is the payload's
/// own JSON, with nothing wrapped around it.
///
/// Which is why the payload carries the derives and this does not. A
/// frame type that could be serialized would be a frame type that had
/// started describing the wire twice.
///
/// # A struct, not an enum
///
/// There is one thing it can hold, so there is nothing to choose
/// between — the same reason
/// [`postgres::Frame`](crate::endpoints::mcp_plugin::client::channel_response::postgres::Frame)
/// is one. A single-variant enum would ask every reader to match on a
/// decision that has already been made.
///
/// If a second thing to stream here ever appears, this becomes an enum
/// then. That is a breaking change, and the honest one: carrying a
/// variant nobody uses against a need that may never come is a cost
/// paid every day for a day that may not arrive.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame(
    /// One chunk of the answer to the client's request.
    pub AgenticLoopChunk,
);

/// The chunk's own JSON, with nothing wrapped around it. A frame type
/// is dispatch rather than a wire shape, so encoding one means
/// encoding what it holds — the newtype leaves no trace on the wire.
impl Encode for Frame {
    /// The ordinary JSON failure. A chunk is entirely shapes: content,
    /// reasoning, tool calls, usage. Nothing in it is a passthrough.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, &self.0)
    }
}

impl Decode<'_> for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes).map(Frame)
    }
}
