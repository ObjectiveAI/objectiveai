//! One MCP request, whole.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::Method;
use crate::encode::{Encode, Writer};

/// An MCP request, complete in a single frame.
///
/// It fits in one because an MCP request body is always a whole JSON
/// document of known length: a [`Post`](Method::Post) carries one
/// JSON-RPC message the agent had finished composing before it sent
/// anything, and [`Get`](Method::Get) and [`Delete`](Method::Delete)
/// carry nothing at all. Nothing here arrives in pieces, so nothing
/// here needs continuing.
///
/// The response is the opposite, and that asymmetry is the whole
/// reason the two directions are shaped differently — see
/// [`response::Frame`](super::super::response::Frame).
///
/// Borrows from the buffer it was decoded out of. A request's whole
/// life is one exchange, so copying its body to own it would be
/// copying it to throw away.
///
/// [`PartialEq`] is written out rather than derived, because
/// [`RawValue`] does not implement it. The hand-written one compares
/// bodies as TEXT, so two requests carrying the same JSON written
/// differently — a space after a colon — are not equal. That is the
/// right reading for a type whose whole promise is that the bytes come
/// out the way they went in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request<'a> {
    /// What the request is doing. This, not [`path`](Self::path), is
    /// the type discriminator.
    pub method: Method,
    /// The request target, RELATIVE — the specifier and any query
    /// string, with no scheme and no authority.
    ///
    /// Never a whole URL. The address the agent dialled is a conduit's
    /// own, a loopback port inside a container: it identifies nothing
    /// outside that container and would be actively misleading
    /// anywhere else. So the base is not carried, and the far end
    /// supplies its own — resolving this against the upstream MCP
    /// server's base URL, the way any reverse proxy substitutes one
    /// origin for another.
    ///
    /// Which leaves this carrying only what the agent's base URL did
    /// not already say. Where the upstream base names the MCP endpoint
    /// exactly, that is nothing, and this is empty. Where a conduit
    /// fronts several MCP servers on one port, it is whatever
    /// distinguishes them.
    ///
    /// The query rides along because some servers use it, and dropping
    /// it would be a silent corruption rather than a visible one.
    pub path: String,
    /// The request headers, verbatim.
    ///
    /// Read only by the far terminator, which is the thing that
    /// actually speaks MCP. `Mcp-Session-Id` is the important one —
    /// Streamable HTTP is session-oriented rather than
    /// connection-oriented, so this header, not any property of a
    /// channel or a socket, is what ties a series of exchanges into
    /// one session.
    ///
    /// A map, so a header name appears at most once. MCP has no use
    /// for repeated names, and this direction never carries the one
    /// header that classically needs them.
    ///
    /// `Origin` is NOT validated at the far end. The MCP spec has
    /// servers check it to defend against DNS rebinding, a threat to
    /// BROWSERS; there is none here, the value only ever names the
    /// agent's own loopback address, and the protocol carrying this is
    /// already the trust boundary. A terminator that enforced it would
    /// reject every honest request.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    /// The JSON-RPC message, for a [`Post`](Method::Post). `None` for
    /// the methods that have no body.
    ///
    /// Raw, and never parsed in transit. Nothing carrying this is an
    /// MCP implementation, and a relay that parsed what it carried
    /// could only fail on what its schema was too old to know, drop
    /// fields it did not model, and hand on bytes that were not the
    /// ones it was given. MCP versions its spec by date and keeps
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

impl PartialEq for Request<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.method == other.method
            && self.path == other.path
            && self.headers == other.headers
            && self.body.map(RawValue::get) == other.body.map(RawValue::get)
    }
}

impl Encode for Request<'_> {
    /// The ordinary JSON failure and nothing else.
    ///
    /// [`body`](Self::body) cannot contribute one: a [`RawValue`] is
    /// already-valid JSON and serializes by being copied out.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}
