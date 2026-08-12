//! What a client's response frame carries on an MCP channel.

use super::McpResponseHead;

/// The payload of a
/// [`ClientFrame::Response`](crate::frame::client::ClientFrame::Response)
/// on a channel opened by
/// [`ServerRequestFrame::Mcp`](super::super::server::ServerRequestFrame::Mcp).
///
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
pub enum ClientMcpResponseFrame<'a> {
    /// The status and headers. Always first, and never repeated.
    Head(McpResponseHead),
    /// A piece of the response body: the whole of it for a single JSON
    /// answer, or one event's worth for a stream.
    Body(&'a [u8]),
}
