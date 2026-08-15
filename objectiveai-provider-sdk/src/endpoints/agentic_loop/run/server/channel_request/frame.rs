//! What a server's request frame carries.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// The payload of a [`ServerFrame::ChannelRequest`](crate::frame::server::ServerFrame::ChannelRequest).
///
/// One MCP exchange, toward the client's MCP proxy. Complete in this
/// frame; the answer comes back as client response frames.
///
/// A payload leads with one byte and the rest is the request.
///
/// It is the one thing a server asks its client for, and it is a
/// connection the server cannot make itself: the agent runs beside the
/// provider, and the MCP servers live with the client. So the provider
/// opens a channel, and the client splices the far end into the real
/// thing.
///
/// # MCP is carried as exchanges, not as a socket
///
/// Because MCP over Streamable HTTP is not a connection. It is a
/// series of discrete exchanges over a session identified by a HEADER
/// rather than by anything at the transport layer.
///
/// Terminating the HTTP at each end and carrying the exchange itself
/// keeps HTTP/1.1 framing out of this protocol entirely: no chunked
/// encoding, no keep-alive boundaries, no request parser in the
/// conduit, and a terminator that can rebuild an ordinary request and
/// hand it to an ordinary router. The JSON-RPC inside stays opaque
/// regardless — see
/// [`Request::body`](crate::shared::http::request::Request::body).
///
/// # A struct, and still a tag byte
///
/// One thing to ask for is a struct; an enum of one variant would be a
/// discriminant with nothing to discriminate.
///
/// The byte stays anyway, for the same reason
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// spends one: a second thing to ask a client for is additive if there
/// is a tag to add to, and a wire break if there is not. Whoever adds
/// one turns this into an enum with `Mcp` at tag `0` and changes
/// nothing on the wire.
///
/// The frame's own `type` could have carried the discrimination — it
/// is right there in the header — and deliberately does not. A frame
/// already carries one payload's worth of protocol; splitting it
/// across the envelope and the payload would mean two vocabularies to
/// version and two places to keep in step.
///
/// # Not the database
///
/// A [`plugin`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Postgres)
/// gets that channel, because a plugin is what needs a database. An
/// agent talks to its tools; a tool is what keeps something. So the
/// tunnel ends where the tool runs, and this loop never sees a
/// connection it has no query to send down.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame<'a>(
    /// The request, relayed verbatim.
    ///
    /// Handed off to
    /// [`Request`](crate::shared::http::request::Request)'s own impl
    /// rather than serialized here — not to save the four lines, but
    /// because an MCP request has ONE wire form, and writing it a
    /// second time in a second place is how two wire forms start.
    pub Request<'a>,
);

/// The tag that says this is an MCP exchange.
const MCP: u8 = 0;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[MCP]);
        self.0.encode(out)
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != MCP {
            return Err(FrameError::UnknownTag(*tag));
        }
        Request::decode(rest).map(Frame).map_err(FrameError::Mcp)
    }
}

/// A server request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag this version does not define.
    ///
    /// Which is what a server asking for something else will send,
    /// once there is something else to ask for. Until then it is a
    /// peer that disagrees about the protocol.
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
