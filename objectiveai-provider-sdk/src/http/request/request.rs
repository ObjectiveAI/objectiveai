//! One request, whole.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::Method;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A tunneled HTTP request, complete in a single frame.
///
/// It fits in one because the requests carried here are whole when
/// they are sent. An MCP `POST` carries one JSON-RPC message the agent
/// had finished composing before it sent anything; a registry `GET` or
/// `HEAD` carries nothing at all. Nothing arrives in pieces, so
/// nothing needs continuing.
///
/// The response is the opposite, and that asymmetry is the whole
/// reason the two directions are framed differently — see
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
    /// Never a whole URL. The address the sender dialled belongs to
    /// whatever conduit it reached, which is a loopback port on the
    /// far side of a container boundary: it identifies nothing outside
    /// that boundary and would be actively misleading anywhere else.
    /// So the base is not carried, and the terminator supplies its own
    /// — resolving this against the upstream it chose, the way any
    /// reverse proxy substitutes one origin for another.
    ///
    /// Which leaves this carrying only what the sender's base URL did
    /// not already say. Against an MCP endpoint that is usually
    /// nothing; against a registry it is the whole `/v2/…` path, and
    /// the repository segment is how a terminator knows which caller a
    /// request belongs to.
    ///
    /// The query rides along because some servers use it, and dropping
    /// it would be a silent corruption rather than a visible one.
    pub path: String,
    /// The request headers, verbatim.
    ///
    /// Read only by the far terminator, which is the thing that
    /// actually speaks the protocol. They are load-bearing in both
    /// directions this carries: `Mcp-Session-Id` is what ties a series
    /// of MCP exchanges into one session, since Streamable HTTP is
    /// session-oriented rather than connection-oriented, and `Range`
    /// is what lets an interrupted blob resume rather than restart.
    ///
    /// A map, so a header name appears at most once. Neither protocol
    /// here repeats one on a request.
    ///
    /// `Origin` is NOT validated at the far end. The specs that ask
    /// for it are defending against DNS rebinding, a threat to
    /// BROWSERS; there is none here, the value only ever names a
    /// loopback address on the sending side, and the protocol carrying
    /// this is already the trust boundary. A terminator that enforced
    /// it would reject every honest request.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    /// The body, for the methods that have one. `None` otherwise.
    ///
    /// JSON, and only JSON — which is what both protocols carried here
    /// need, and a limit worth knowing before a third arrives. A
    /// [`RawValue`] nests into the envelope with no base64 and no
    /// re-serialization, and comes out the far side byte-identical;
    /// bytes would cost that and buy a body nothing currently sends.
    ///
    /// Raw, and never parsed in transit. Nothing carrying this is an
    /// implementation of what it carries, and a relay that parsed
    /// could only fail on what its schema was too old to know, drop
    /// fields it did not model, and hand on bytes that were not the
    /// ones it was given. MCP versions its spec by date and keeps
    /// adding extensions; JSON-RPC batching was required in one
    /// version and removed in the next. None of that is this layer's
    /// business.
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
    /// The ordinary JSON failure.
    ///
    /// [`body`](Self::body) cannot contribute one: a [`RawValue`] is
    /// already-valid JSON and serializes by being copied out.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl<'a> Decode<'a> for Request<'a> {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
