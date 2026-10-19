//! What a client's channel request frame carries on an accept scope.

use std::fmt;

use crate::shared::containers::daemon;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What the accepting daemon asks the provider for while it accepts.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Connection`](Self::Connection) |
/// | `1` | [`Stop`](Self::Stop) |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// The daemon's half of a connection the provider announced,
    /// quoting the id. Tag `0`.
    ///
    /// What comes back on it is everything the connector sends — its
    /// client frames, one per channel response — and its finish is
    /// the connector gone. Opening it is taking the connection; the
    /// provider's half, finished with nothing, is declining it.
    Connection(daemon::request::Daemon),
    /// Stop accepting. Tag `1`.
    ///
    /// Carries nothing. Nothing answers on the channel; the scope
    /// finishes, and every connection relayed on it ends.
    Stop,
}

/// Tag for [`Frame::Connection`].
const CONNECTION: u8 = 0;

/// Tag for [`Frame::Stop`].
const STOP: u8 = 1;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): known bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Connection(connection) => {
                out.extend_from_slice(&[CONNECTION]);
                connection.encode(out)
            }
            Frame::Stop => {
                out.extend_from_slice(&[STOP]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CONNECTION => daemon::request::Daemon::decode(rest).map(Frame::Connection).map_err(FrameError::Connection),
            STOP => Ok(Frame::Stop),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An accept channel request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The connection id was not four bytes.
    Connection(daemon::request::DaemonError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("daemons accept channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown daemons accept channel request tag {tag}"),
            FrameError::Connection(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Connection(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
