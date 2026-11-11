//! What a server's response frame carries for an accept.

use std::fmt;

use super::Accepting;
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// An accept's answer: the daemon's identity, and then nothing for as
/// long as the scope lives — or a refusal, or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Accepting`](Self::Accepting), `1` for [`Error`](Self::Error), `2`
/// for [`AlreadyOpen`](Self::AlreadyOpen) — and the rest is that
/// variant's own JSON, or nothing for `AlreadyOpen`.
///
/// | the scope | means |
/// |-----------|-------|
/// | an identity, then nothing, and stays open | the daemon accepts; every connection arrives as a channel |
/// | already open, then a finish | it does not: this connection, or another under the identity, accepts here already |
/// | an error, then a finish | it does not: the provider could not serve it |
/// | a finish, with no error | the accept is over — the cancel, or the connection |
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The daemon accepts, under this identity. Tag `0`.
    Accepting(Accepting),
    /// A failure. Tag `1`.
    ///
    /// Ends the scope rather than adding to it. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
    /// Refused: this connection has an accept open already, or another
    /// connection under the same identity has. Tag `2`.
    ///
    /// Carries nothing. Sent first and alone, and the finish follows
    /// it: nothing is accepted. See the
    /// [endpoints](crate::provider::endpoints#one-open-at-a-time).
    AlreadyOpen,
}

/// Tag for [`Frame::Accepting`].
const ACCEPTING: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Tag for [`Frame::AlreadyOpen`].
const ALREADY_OPEN: u8 = 2;

impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Accepting(accepting) => {
                out.extend_from_slice(&[ACCEPTING]);
                serde_json::to_writer(out, accepting)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
            Frame::AlreadyOpen => {
                out.extend_from_slice(&[ALREADY_OPEN]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and each names which variant failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ACCEPTING => serde_json::from_slice(rest).map(Frame::Accepting).map_err(FrameError::Accepting),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            ALREADY_OPEN => Ok(Frame::AlreadyOpen),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An accept response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The identity did not parse.
    Accepting(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("daemons accept response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown daemons accept response frame tag {tag}"),
            FrameError::Accepting(error) => write!(f, "daemons accept identity did not parse: {error}"),
            FrameError::Error(error) => write!(f, "daemons accept error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Accepting(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
