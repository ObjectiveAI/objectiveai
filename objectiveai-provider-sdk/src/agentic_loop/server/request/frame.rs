//! What a server's request frame carries.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::http::request::Request;

/// The payload of a [`ServerFrame::ChannelRequest`](crate::frame::server::ServerFrame::ChannelRequest).
///
/// A server asks its client for two things, and both are the same ask
/// in different clothes: a connection it cannot make itself. The agent
/// runs beside the provider; the MCP servers and the database live
/// with the client. So the provider opens a channel, and the client
/// splices the far end into the real thing.
///
/// Which variant applies is the frame's own type, not anything in the
/// payload — the server's type space is open above `4` for exactly
/// this. Once a channel is open its kind is settled, and what comes
/// back on it needs no tag at all.
///
/// # Why the two are shaped differently
///
/// [`Postgres`](Self::Postgres) is a byte stream and
/// [`Mcp`](Self::Mcp) is a structured exchange, because pgwire really
/// is a CONNECTION and MCP over Streamable HTTP really is not.
///
/// A Postgres session is a long-lived socket carrying a conversation
/// with no natural top-level unit, so successive request frames on one
/// channel are successive writes, and a message larger than one frame
/// simply spans several. Never parsing it is what lets TLS negotiation
/// and every protocol extension cross untouched — the argument
/// db-proxy's conduit already makes.
///
/// MCP is a series of discrete exchanges over a session identified by
/// a HEADER rather than by any connection. Terminating the HTTP at
/// each end and carrying the exchange itself keeps HTTP/1.1 framing
/// out of this protocol entirely: no chunked encoding, no keep-alive
/// boundaries, no request parser in the conduit, and a terminator that
/// can rebuild an ordinary request and hand it to an ordinary router.
/// The JSON-RPC inside stays opaque regardless — see
/// [`mcp::request::Request::body`](crate::http::request::Request::body).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One MCP exchange, toward the client's MCP proxy. Complete in
    /// this frame; the answer comes back as client response frames.
    Mcp(Request<'a>),
    /// Postgres bytes, toward the database. Opaque, and a stream —
    /// this is a socket, and successive frames on the channel are
    /// successive writes.
    Postgres(&'a [u8]),
}

/// Tag for [`Frame::Mcp`].
const MCP: u8 = 0;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 1;

/// A payload leads with one byte saying which variant it is, and the
/// rest is that variant's own bytes.
///
/// The frame's own `type` could have carried this — it is right there
/// in the header — and deliberately does not. A frame already carries
/// one payload's worth of protocol; splitting the discrimination
/// across the envelope and the payload would mean two vocabularies to
/// version and two places to keep in step. Here the whole of what a
/// channel carries is described in one place, and the frame layer
/// stays ignorant of it.
///
/// [`Mcp`](Frame::Mcp) hands off to
/// [`Request`](crate::http::request::Request)'s own impl rather than
/// serializing the request here. Not to save the four lines — because
/// an MCP request has ONE wire form, and writing it a second time in
/// a second place is how two wire forms start.
impl Encode for Frame<'_> {
    /// The MCP request's error, since the other variant has none.
    /// Postgres bytes are copied, and copying cannot fail — so the
    /// union of the two is just what MCP can do wrong.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Mcp(request) => {
                out.extend_from_slice(&[MCP]);
                request.encode(out)
            }
            Frame::Postgres(bytes) => {
                out.extend_from_slice(&[POSTGRES]);
                out.extend_from_slice(bytes);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            MCP => Request::decode(rest)
                .map(Frame::Mcp)
                .map_err(FrameError::Mcp),
            POSTGRES => Ok(Frame::Postgres(rest)),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A server request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from a zero-length write, which is a tag byte followed
    /// by nothing and is ordinary on a socket.
    Empty,
    /// A tag byte that is neither [`Frame::Mcp`] nor
    /// [`Frame::Postgres`].
    UnknownTag(u8),
    /// The MCP request did not parse.
    Mcp(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("server request frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown server request frame tag {tag}")
            }
            FrameError::Mcp(error) => {
                write!(f, "mcp request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Mcp(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
