//! What a server's response frame carries for a tool container begin.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A begin's answer: that the connection has begun, and then nothing,
/// for as long as the connection lives — or a failure.
///
/// A payload leads with one byte saying which — `0` for
/// [`Begun`](Self::Begun), `1` for [`Error`](Self::Error) — and only
/// the error carries anything after it.
///
/// # Silence is the good case
///
/// | the scope | means |
/// |-----------|-------|
/// | a begun, then nothing, and stays open | the connection has begun; either side may open channels on it |
/// | an error, then a finish | it has not — this connection had already begun |
/// | a finish, with no error | the proxy is ending |
///
/// Everything the server reads from the container — the family's own
/// exchange, the asks the container makes — is a channel, not this
/// stream. It carries no readiness signal beyond the one word: the
/// proxy is here, and channels may be opened.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The connection has begun. Tag `0`.
    ///
    /// Arrives once, at once. A channel on this scope is opened only
    /// after it.
    Begun,
    /// A failure. Tag `1`.
    ///
    /// A second begin on a connection that had one. It is the one
    /// variant that ends the scope rather than adding to it. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Begun`].
const BEGUN: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. `Begun` cannot fail at all.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Begun => {
                out.extend_from_slice(&[BEGUN]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BEGUN => Ok(Frame::Begun),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tools begin response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools begin response frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools begin response frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "tools begin error did not parse: {error}")
            }
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
