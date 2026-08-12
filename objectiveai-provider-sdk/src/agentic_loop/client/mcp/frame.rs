//! What a client's response frame carries on an MCP channel.

use indexmap::IndexMap;

/// The payload of a
/// [`ClientFrame::Response`](crate::frame::client::ClientFrame::Response)
/// on a channel opened by
/// [`RequestFrame::Mcp`](super::super::super::server::RequestFrame::Mcp).
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
pub enum Frame<'a> {
    /// The status and headers. Always first, and never repeated.
    ///
    /// On the wire this is a JSON object of exactly these two fields.
    Head {
        /// The HTTP status.
        ///
        /// Load-bearing, and the reason a bare JSON-RPC message would
        /// not do: `202` marks a notification that has no body coming,
        /// and `404` tells the agent its session is gone and must be
        /// re-initialized. Neither fact has anywhere to live inside
        /// JSON-RPC.
        status: u16,
        /// The response headers, verbatim.
        ///
        /// Also load-bearing. `Mcp-Session-Id` is how an agent LEARNS
        /// its session id in the first place — the initialize response
        /// mints it — and `Content-Type` is what tells the agent
        /// whether it is reading one JSON document or an event stream.
        ///
        /// A map, so a header name appears at most once. This is the
        /// direction where that could bite, since `Set-Cookie` is the
        /// classic repeated header; MCP does not use cookies, and the
        /// ergonomics everywhere else are worth more than the case.
        headers: IndexMap<String, String>,
    },
    /// A piece of the response body: the whole of it for a single JSON
    /// answer, or one event's worth for a stream.
    Body(&'a [u8]),
}
