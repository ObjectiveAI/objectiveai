//! What a response frame carries on an MCP channel.

use super::Head;
use crate::encode::{Encode, Writer};

/// One MCP answer, arriving in pieces: the head once, then as much
/// body as there turns out to be.
///
/// # How a reader tells them apart
///
/// By POSITION. The first response frame on the channel is the
/// [`Head`](Self::Head); every one after it is [`Body`](Self::Body).
/// That ordering is the whole encoding — there is no type byte on a
/// response frame, no length, no count, and no terminator beyond the
/// channel's finish.
///
/// # Why the split
///
/// Because an MCP answer is not finished when it starts, and the
/// request that provoked it was. A `POST` may be answered with one
/// JSON document or with an event stream held open while the far
/// server works; a `GET` is answered with a stream held open for the
/// whole SESSION. Neither end knows which in advance.
///
/// Neither end needs to. Both are this same sequence, differing only
/// in how many bodies there are and how far apart they land — a single
/// JSON answer is a stream that ended after one. So there is no mode
/// to negotiate and no flag to carry: the one place the difference is
/// stated is `Content-Type` in the head's headers, which is a header
/// being relayed anyway.
///
/// Sending the head first is also what lets a conduit avoid buffering.
/// It can write the status line and headers onto the agent's socket
/// the moment the head arrives, then pump bodies straight through.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// The status and headers. Always first, and never repeated.
    Head(Head),
    /// A piece of the response body: the whole of it for a single JSON
    /// answer, or one event's worth for a stream.
    Body(&'a [u8]),
}

/// Two variants, two encodings — which is the point of the format
/// living in the type. [`Head`](Frame::Head) is a JSON object because
/// it is a shape someone reads; [`Body`](Frame::Body) is written
/// through untouched because it is whatever the far MCP server said,
/// and re-encoding it would break the byte-identity the tunnel exists
/// to preserve.
impl Encode for Frame<'_> {
    /// Only [`Head`](Frame::Head) can fail, and only the way any JSON
    /// serialization can. A body is bytes and has nothing to get
    /// wrong.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Head(head) => serde_json::to_writer(out, head),
            Frame::Body(body) => {
                out.extend_from_slice(body);
                Ok(())
            }
        }
    }
}

// No `Decode`. This enum is a CHOICE, and the bytes do not contain it:
// head or body is settled by POSITION on the channel, which is the
// reader's own state rather than anything in a payload. A reader that
// knows it holds the first response frame decodes a
// [`Head`](crate::mcp::response::Head) and wraps it; every one after
// is `Frame::Body(bytes)` with nothing to parse.
//
// Which is the asymmetry between the two traits. `Encode` on a
// dispatch enum is fine — you know which variant you are holding.
// `Decode` is not, because knowing is exactly what the bytes cannot
// tell you.
