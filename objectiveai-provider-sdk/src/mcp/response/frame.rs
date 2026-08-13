//! What a response frame carries on an MCP channel.

use std::error::Error;
use std::fmt;

use super::Head;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// One MCP answer, arriving in pieces: the head once, then as much
/// body as there turns out to be.
///
/// # The tag byte
///
/// A payload leads with one byte saying which variant it is — `0` for
/// [`Head`](Self::Head), `1` for [`Body`](Self::Body) — and the rest
/// is that variant's own bytes.
///
/// The alternative was position: first frame on the channel is the
/// head, every one after is body. That needs no byte and is worse.
/// It makes a frame mean something only in the context of the ones
/// before it, so a reader has to carry state to interpret one, and a
/// frame that arrives out of order or after a gap is not detectably
/// wrong — it is silently the other thing. A tag costs a byte per
/// frame on a stream whose frames are HTTP bodies, which is nothing,
/// and buys a payload that can be read on its own.
///
/// It is also what lets this implement [`Decode`]. A dispatch enum
/// whose discriminator lives outside the payload cannot: `decode` is
/// a function of the bytes, so a choice the bytes do not contain is a
/// choice it cannot make.
///
/// # Why the split at all
///
/// Because an MCP answer is not finished when it starts, and the
/// request that provoked it was. A `POST` may be answered with one
/// JSON document or with an event stream held open while the far
/// server works; a `GET` is answered with a stream held open for the
/// whole SESSION. Neither end knows which in advance.
///
/// Neither end needs to. Both are the same sequence, differing only in
/// how many bodies there are and how far apart they land — a single
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
    /// The status and headers. Tag `0`. Always first, and never
    /// repeated.
    Head(Head),
    /// A piece of the response body: the whole of it for a single JSON
    /// answer, or one event's worth for a stream. Tag `1`.
    Body(&'a [u8]),
}

/// Tag for [`Frame::Head`].
const HEAD: u8 = 0;

/// Tag for [`Frame::Body`].
const BODY: u8 = 1;

/// Two variants, two encodings — which is the point of the format
/// living in the type. [`Head`](Frame::Head) is a JSON object because
/// it is a shape someone reads; [`Body`](Frame::Body) is written
/// through untouched because it is whatever the far MCP server said,
/// and re-encoding it would break the byte-identity the tunnel exists
/// to preserve. The tag in front is the only thing either gains.
impl Encode for Frame<'_> {
    /// Only [`Head`](Frame::Head) can fail, and only the way any JSON
    /// serialization can. A body is bytes and has nothing to get
    /// wrong.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Head(head) => {
                out.extend_from_slice(&[HEAD]);
                serde_json::to_writer(out, head)
            }
            Frame::Body(body) => {
                out.extend_from_slice(&[BODY]);
                out.extend_from_slice(body);
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
            HEAD => serde_json::from_slice(rest)
                .map(Frame::Head)
                .map_err(FrameError::Head),
            BODY => Ok(Frame::Body(rest)),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An MCP response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from a body of length zero, which is a tag byte
    /// followed by nothing and is perfectly ordinary — an event stream
    /// can carry an empty chunk.
    Empty,
    /// A tag byte that is neither [`Frame::Head`] nor [`Frame::Body`].
    ///
    /// An error rather than something to skip. Unlike the frame
    /// TYPE space, where a value this layer does not know is a newer
    /// peer's request and can be passed along unread, there is no
    /// third thing an MCP response can be — so an unfamiliar tag here
    /// is a peer that disagrees about the protocol, not one that has
    /// more of it.
    UnknownTag(u8),
    /// The head did not parse.
    Head(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("mcp response frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp response frame tag {tag}")
            }
            FrameError::Head(error) => {
                write!(f, "mcp response head did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Head(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
