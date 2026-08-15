//! What a server's response frame carries for a connection.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 0;

/// Tag for [`Frame::Connections`].
const CONNECTIONS: u8 = 1;

/// Two variants, two encodings, and neither converted into the
/// other's. A count is four big-endian bytes, the same way `scope` and
/// `channel` are written in every header; a filetree frame is
/// postcard's and is handed to postcard.
impl Encode for Frame {
    /// Postcard's, since only the filetree half can fail. A
    /// fixed-width integer has no failure mode.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Filetree(frame) => {
                out.extend_from_slice(&[FILETREE]);
                frame.encode(out)
            }
            Frame::Connections(count) => {
                out.extend_from_slice(&[CONNECTIONS]);
                out.extend_from_slice(&count.to_be_bytes());
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            FILETREE => crate::shared::filetree::response::Frame::decode(rest)
                .map(Frame::Filetree)
                .map_err(FrameError::Filetree),
            CONNECTIONS => <[u8; 4]>::try_from(rest)
                .map(|bytes| Frame::Connections(u32::from_be_bytes(bytes)))
                .map_err(|_| FrameError::Connections(rest.len())),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A connection response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The filetree frame did not decode.
    Filetree(postcard::Error),
    /// A connection count that was not four bytes, carrying however
    /// many there were.
    Connections(usize),
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
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Filetree(error) => Some(error),
            FrameError::Empty
            | FrameError::UnknownTag(_)
            | FrameError::Connections(_) => None,
        }
    }
}
