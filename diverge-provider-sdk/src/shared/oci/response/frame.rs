//! What a registry answer carries back.

use std::error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A piece of the registry's answer, or the reason there will not be
/// another.
///
/// A payload leads with one byte saying which — `0` for
/// [`Body`](Self::Body), `1` for [`Error`](Self::Error) — and the rest
/// is that variant's own bytes.
///
/// # Many frames, and no head
///
/// The answer is a status line, headers and a body, and all three are
/// [`Body`](Self::Body) frames — because all three are bytes off a
/// socket, and where one ends and the next begins is HTTP's business
/// rather than this channel's.
///
/// There used to be a head, carrying a status and a header map. It went
/// with the parsing: a relay that hands over a status has read one, and
/// a relay that has read one has to decide what to do about the headers
/// that describe the MESSAGE rather than the answer. Every such
/// decision was a bug — a `Content-Length` copied onto a body that had
/// been re-encoded, a `Connection` forwarded past the hop it belonged
/// to.
///
/// So the pieces mean nothing and are not required to align with
/// anything. A caller sends what it has when it has it; a provider
/// writes it onto the runtime's socket in the order it arrives. That is
/// what makes a layer of hundreds of megabytes possible without either
/// end holding one.
///
/// # An error is the last thing on it
///
/// And it is the PROVIDER's kind of failure rather than the registry's.
/// A registry that refuses says so in HTTP — a `404`, a `401`, an
/// error document — and that travels as [`Body`](Self::Body) like any
/// other answer, because it IS the answer.
///
/// [`Error`](Self::Error) is for when there is no answer to relay: the
/// caller could not reach its registry, or stopped being able to. A
/// runtime seeing a connection close mid-body would learn the same
/// thing eventually; this says it now, and says why.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Bytes of the answer, borrowed from the frame they arrived in.
    /// Tag `0`.
    Body(&'a [u8]),
    /// There will be no more, and this is why. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Body`].
const BODY: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the only variant that has one.
    /// Bytes are copied.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Body(bytes) => {
                out.extend_from_slice(&[BODY]);
                out.extend_from_slice(bytes);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BODY => Ok(Frame::Body(rest)),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A registry answer that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from an empty body, which is a tag byte followed by
    /// nothing and is ordinary on a socket.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("registry answer frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown registry answer frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "registry answer error did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
