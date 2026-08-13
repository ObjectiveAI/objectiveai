//! One MCP response, whole.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// An MCP response.
///
/// The whole answer: what a caller would have if it had waited for the
/// exchange to finish. That is a useful thing to be able to say and a
/// bad way to send it — a `POST` may be answered with an event stream
/// held open while the far server works, and a `GET` with one held
/// open for the length of the session, so anything that waited for
/// [`body`](Self::body) to be complete would be buffering a stream it
/// was meant to be relaying.
///
/// Hence the split on the wire and not here. This module is the
/// exchange; how it is cut into frames belongs to whatever carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Response {
    /// The HTTP status.
    ///
    /// Load-bearing, and the reason a bare JSON-RPC message would not
    /// do: `202` marks a notification that has no body coming, and
    /// `404` tells an agent its session is gone and must be
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
    /// The body, verbatim.
    ///
    /// Bytes rather than JSON, because this is not always JSON. When
    /// `Content-Type` is `text/event-stream` it is a sequence of SSE
    /// events, and even when it is one JSON document, nothing carrying
    /// it has any business parsing it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<u8>,
}
