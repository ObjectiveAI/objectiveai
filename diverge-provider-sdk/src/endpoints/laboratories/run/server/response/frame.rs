//! What a server's response frame carries for a laboratory run.

use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A run's answer: the container's id, how many connectors are on
/// it, and its filesystem, for as long as the scope lives.
///
/// # The tag byte
///
/// A payload leads with one byte saying which variant it is — `0` for
/// [`Id`](Self::Id), `1` for [`Filetree`](Self::Filetree), `2` for
/// [`Connections`](Self::Connections) — and the rest is that variant's
/// own bytes. The same arrangement
/// [`http::response::Frame`](crate::shared::http::response::Frame) uses, and
/// for the same reason: a frame that means something only in the
/// context of the ones before it needs a reader carrying state, and
/// one arriving out of order is not detectably wrong, it is silently
/// the other thing.
///
/// # Which is why nothing here is ordered
///
/// The three kinds interleave however a provider produces them. An id
/// may land before the first filesystem frame, after the snapshot, or
/// somewhere among the deltas — a container can be running and
/// reporting before its provider has finished deciding what to call
/// it — and a connector can arrive at any moment, which is not a
/// moment anything else is waiting for.
///
/// A reader takes each frame as it comes and does not count. That is
/// what the tag bought, and it paid for itself here: had the split
/// been positional, an order would have had to be invented and then
/// obeyed by every provider forever.
///
/// It costs a byte per filetree frame, which is the bulk of the
/// traffic. That is the price of frames that can be read on their own,
/// and it is small next to the paths they carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// The container's id. Tag `0`.
    ///
    /// Arrives whenever the provider has it, which is not necessarily
    /// before the filesystem starts reporting.
    ///
    /// What it is for is between the two ends. What it is FROM is this
    /// scope: a provider mints it here, and a caller that wants to
    /// name this container anywhere else has this and nothing else to
    /// name it with.
    Id(String),
    /// One change on the container's filesystem. Tag `1`.
    ///
    /// A [`filetree`](crate::shared::filetree) stream over the container's own
    /// root — one snapshot, then one frame per change — which is the
    /// same thing
    /// [`filesystem::watch`](crate::endpoints::volumes::watch) answers with,
    /// over a different tree.
    ///
    /// Which is why running and watching are not two asks. A
    /// container's filesystem is the observable part of it running, so
    /// the scope that made the container is the scope that reports on
    /// it.
    Filetree(crate::shared::filetree::response::Frame),
    /// How many connectors are attached to the container. Tag `2`.
    ///
    /// Sent again whenever the number changes, which makes this a
    /// value rather than an event: a reader holds the last one it saw
    /// and needs no arithmetic, so a frame lost or replayed leaves it
    /// with a number rather than a drift.
    ///
    /// # What the authorize channel is for
    ///
    /// A connector arriving is what prompts
    /// [`Authorize`](super::super::channel_request::Frame::Authorize).
    /// The provider asks the caller whether this one may attach, the
    /// caller answers yes or no, and a yes is what this count then
    /// reflects.
    ///
    /// So the two are halves of one exchange, and the order between
    /// them is the only order in this stream that means anything: the
    /// question is asked, then the answer changes the number. Nothing
    /// enforces it — a reader that saw the count move without having
    /// answered is looking at a provider that decided on its own.
    Connections(u32),
}

/// Tag for [`Frame::Id`].
const ID: u8 = 0;

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 1;

/// Tag for [`Frame::Connections`].
const CONNECTIONS: u8 = 2;

/// Three variants, three encodings, and none converted into another's
/// to make them match. An id is a string and goes out as its own
/// bytes; a count is four big-endian bytes, the same way `scope` and
/// `channel` are written in every header; a filetree frame is
/// postcard's and is handed to postcard.
impl Encode for Frame {
    /// Postcard's, since only the filetree half can fail. A string's
    /// bytes and a fixed-width integer have no failure mode.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Id(id) => {
                out.extend_from_slice(&[ID]);
                out.extend_from_slice(id.as_bytes());
                Ok(())
            }
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
    /// Five ways to fail, and each names which half failed.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ID => String::from_utf8(rest.to_vec())
                .map(Frame::Id)
                .map_err(FrameError::Id),
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

/// A laboratory run response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The id was not UTF-8.
    Id(FromUtf8Error),
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
                f.write_str("laboratory run response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown laboratory run response frame tag {tag}")
            }
            FrameError::Id(error) => {
                write!(f, "container id is not utf-8: {error}")
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
            FrameError::Id(error) => Some(error),
            FrameError::Filetree(error) => Some(error),
            FrameError::Empty
            | FrameError::UnknownTag(_)
            | FrameError::Connections(_) => None,
        }
    }
}
