//! One MCP request, whole.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::McpMethod;

/// An MCP request, complete in a single frame.
///
/// It fits in one frame because an MCP request body is always a whole
/// JSON document of known length: a `POST` carries one JSON-RPC
/// message the agent had finished composing before it sent anything,
/// and `GET` and `DELETE` carry nothing at all. Nothing here arrives
/// in pieces, so nothing here needs continuing.
///
/// The response is the opposite, and that asymmetry is the whole
/// reason the two directions have different shapes — see
/// [`McpResponseHead`](super::super::super::client::McpResponseHead).
///
/// [`PartialEq`] is written out rather than derived, because
/// [`RawValue`] does not implement it. The hand-written one compares
/// bodies as TEXT, so two requests carrying the same JSON written
/// differently — a space after a colon — are not equal. That is the
/// right reading for a wire type whose whole promise is that the bytes
/// come out the way they went in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest<'a> {
    /// What the request is doing. This, not [`path`](Self::path), is
    /// the type discriminator.
    pub method: McpMethod,
    /// The request target in origin form — the path, plus any query
    /// string, exactly as it appeared on the request line.
    ///
    /// Carried rather than assumed, for three reasons. A terminator
    /// cannot rebuild a request without one, and synthesizing `/`
    /// would mean inventing the one part of the request it was
    /// supposed to be relaying. MCP's OAuth discovery lives at real,
    /// distinct paths under `/.well-known/`, which a stock agent SDK
    /// may probe whether or not this deployment needs them. And it is
    /// the natural seam if a conduit ever fronts several MCP servers
    /// by path rather than by port.
    ///
    /// The query is included because some servers use it and dropping
    /// it would be a silent corruption rather than a visible one.
    pub path: String,
    /// The request headers, verbatim.
    ///
    /// Sent as they arrived and read only by the far terminator, which
    /// is the thing that actually speaks MCP. `Mcp-Session-Id` is the
    /// important one — Streamable HTTP is session-oriented rather than
    /// connection-oriented, so this header, not any property of the
    /// channel, is what ties a series of exchanges into one session.
    ///
    /// A map, so a header name appears at most once. MCP has no use
    /// for repeated names, and the request direction never carries the
    /// one header that classically needs them.
    ///
    /// `Origin` is NOT validated at the far end. The MCP spec has
    /// servers check it to defend against DNS rebinding, a threat to
    /// BROWSERS; there is none here, the value only ever names the
    /// agent's own loopback address, and this protocol is already the
    /// trust boundary. A terminator that enforced it would reject
    /// every honest request.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    /// The JSON-RPC message, for a [`Post`](McpMethod::Post). `None`
    /// for the methods that have no body.
    ///
    /// Raw, and never parsed in transit. Neither end of this channel
    /// is an MCP implementation, and a relay that parsed what it
    /// carried could only fail on what its schema was too old to know,
    /// drop fields it did not model, and hand on bytes that were not
    /// the ones it was given. MCP versions its spec by date and keeps
    /// adding extensions; JSON-RPC batching was required in one
    /// version and removed in the next. None of that is this layer's
    /// business.
    ///
    /// A [`RawValue`] rather than a byte string because the payload is
    /// JSON already: it nests into the envelope with no base64 and no
    /// re-serialization, and comes out the far side byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none", borrow)]
    pub body: Option<&'a RawValue>,
}

impl PartialEq for McpRequest<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.method == other.method
            && self.path == other.path
            && self.headers == other.headers
            && self.body.map(RawValue::get) == other.body.map(RawValue::get)
    }
}
