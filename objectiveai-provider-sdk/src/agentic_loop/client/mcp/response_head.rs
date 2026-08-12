//! The head of an MCP response.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The status and headers of an MCP response, sent before its body.
///
/// Split from the body because a response — unlike the request that
/// provoked it — is not finished when it starts. A `POST` may be
/// answered with one JSON document or with an event stream held open
/// while the far server works; a `GET` is answered with a stream held
/// open for the whole SESSION. Which of those is happening is not
/// knowable in advance, by either end.
///
/// The protocol does not need to know. Both cases are the same
/// sequence of frames — this head, then body frames, then a finish —
/// and differ only in how many body frames there are and how far apart
/// they arrive. A single JSON answer is just a stream that ended after
/// one. So there is no mode to negotiate and no flag to carry: the one
/// place the difference is stated is `Content-Type` in
/// [`headers`](Self::headers), which is a header being relayed anyway.
///
/// Sending the head early is also what lets a conduit avoid buffering.
/// It can write the status line and headers onto the agent's socket
/// the moment this arrives, then pump body frames straight through as
/// they come.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpResponseHead {
    /// The HTTP status.
    ///
    /// Load-bearing, and the reason a bare JSON-RPC message would not
    /// do: `202` marks a notification that has no body coming, and
    /// `404` tells the agent its session is gone and must be
    /// re-initialized. Neither fact has anywhere to live inside
    /// JSON-RPC.
    pub status: u16,
    /// The response headers, verbatim.
    ///
    /// Also load-bearing. `Mcp-Session-Id` is how an agent LEARNS its
    /// session id in the first place — the initialize response mints
    /// it — and `Content-Type` is what tells the agent whether it is
    /// reading one JSON document or an event stream.
    ///
    /// A map, so a header name appears at most once. This is the
    /// direction where that could bite, since `Set-Cookie` is the
    /// classic repeated header; MCP does not use cookies, and the
    /// ergonomics everywhere else are worth more than the case.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
}
