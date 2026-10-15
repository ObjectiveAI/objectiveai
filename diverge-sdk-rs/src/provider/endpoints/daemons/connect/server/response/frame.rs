//! What a server's response frame carries for a connect.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A connect's answer: the connection open, and then every server
/// frame the acceptor sends on it, one per response, for as long as
/// the scope lives — or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Connected`](Self::Connected), `1` for [`Error`](Self::Error),
/// `2` for [`Frame`](Self::Frame) — and the rest is that variant's
/// own bytes: nothing, the error's JSON, or one
/// [`daemon::server::Frame`](crate::shared::containers::daemon::server::Frame)
/// as the acceptor sent it, forwarded whole.
///
/// | the scope | means |
/// |-----------|-------|
/// | connected, then frames as they come, and stays open | the connection is open |
/// | an error, then a finish | it never was: no daemon accepts under the identity, or it declined |
/// | a finish, with no error | the acceptor hung up, or stopped accepting |
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// The acceptor took the connection. Tag `0`, once, first.
    Connected,
    /// A failure. Tag `1`.
    ///
    /// The one variant that ends the scope rather than adding to it:
    /// `{"kind":"missing"}` for an identity no daemon accepts under
    /// here, `{"kind":"denied"}` for an acceptor that declined. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
    /// One server frame of the acceptor's, after the connected. Tag
    /// `2`.
    Frame(&'a [u8]),
}

/// Tag for [`Frame::Connected`].
const CONNECTED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Tag for [`Frame::Frame`].
const FRAME: u8 = 2;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, of the error alone.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Connected => {
                out.extend_from_slice(&[CONNECTED]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
            Frame::Frame(bytes) => {
                out.extend_from_slice(&[FRAME]);
                out.extend_from_slice(bytes);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CONNECTED => Ok(Frame::Connected),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            FRAME => Ok(Frame::Frame(rest)),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A connect response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("daemons connect response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown daemons connect response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "daemons connect error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
