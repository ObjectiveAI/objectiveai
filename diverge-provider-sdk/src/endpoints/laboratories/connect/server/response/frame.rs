//! What a server's response frame carries for a connection.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A connection's answer: the container's filesystem and its connector
/// count, for as long as the scope lives.
///
/// The same two things a run reports, minus the id — a connector
/// already has that, which is how it got here.
///
/// # The tag byte
///
/// A payload leads with one byte saying which variant it is — `0` for
/// [`Filetree`](Self::Filetree), `1` for
/// [`Connections`](Self::Connections) — and the rest is that variant's
/// own bytes. Nothing is ordered: a reader takes each frame as it
/// comes and does not count.
///
/// The tags start at `0` and have nothing to do with a run's,
/// which happen to number the same kinds differently. Each frame type
/// owns its own tag space; a value means something only inside the
/// type that defines it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One change on the container's filesystem. Tag `0`.
    ///
    /// The same [`filetree`](crate::shared::filetree) stream a run gets,
    /// over the same tree. A connector sees what the runner sees.
    Filetree(crate::shared::filetree::response::Frame),
    /// How many connectors are attached to the container. Tag `1`.
    ///
    /// Sent again whenever the number changes, which makes this a
    /// value rather than an event: a reader holds the last one it saw
    /// and needs no arithmetic, so a frame lost or replayed leaves it
    /// with a number rather than a drift.
    ///
    /// A connector counts itself. The number includes this connection,
    /// so the first one it sees is never zero.
    Connections(u32),
    /// A failure. Tag `2`.
    ///
    /// The connection is not open and will not be — the laboratory was
    /// not there, its runner said no, whatever the provider knows. It
    /// is the one variant that ends the scope rather than adding to
    /// it.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 0;

/// Tag for [`Frame::Connections`].
const CONNECTIONS: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// Two variants, two encodings, and neither converted into the
/// other's. A count is four big-endian bytes, the same way `scope` and
/// `channel` are written in every header; a filetree frame is
/// postcard's and is handed to postcard.
impl Encode for Frame {
    /// One failure per half that has one, and they are different
    /// libraries'. A fixed-width integer has no failure mode.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Filetree(frame) => {
                out.extend_from_slice(&[FILETREE]);
                frame.encode(out).map_err(FrameEncodeError::Filetree)
            }
            Frame::Connections(count) => {
                out.extend_from_slice(&[CONNECTIONS]);
                out.extend_from_slice(&count.to_be_bytes());
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(FrameEncodeError::Error)
            }
        }
    }
}

/// A connection response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The filetree frame did not serialize.
    Filetree(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Filetree(error) => {
                write!(f, "filetree frame did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "connection error did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Filetree(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

impl Decode<'_> for Frame {
    /// Five ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            FILETREE => crate::shared::filetree::response::Frame::decode(rest)
                .map(Frame::Filetree)
                .map_err(FrameError::Filetree),
            CONNECTIONS => <[u8; 4]>::try_from(rest)
                .map(|bytes| Frame::Connections(u32::from_be_bytes(bytes)))
                .map_err(|_| FrameError::Connections(rest.len())),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A connection response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The filetree frame did not decode.
    Filetree(postcard::Error),
    /// A connection count that was not four bytes, carrying however
    /// many there were.
    Connections(usize),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("connection response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown connection response frame tag {tag}")
            }
            FrameError::Filetree(error) => {
                write!(f, "filetree frame did not decode: {error}")
            }
            FrameError::Connections(len) => {
                write!(f, "connection count is {len} bytes, not 4")
            }
            FrameError::Error(error) => {
                write!(f, "connection error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Filetree(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Empty
            | FrameError::UnknownTag(_)
            | FrameError::Connections(_) => None,
        }
    }
}
