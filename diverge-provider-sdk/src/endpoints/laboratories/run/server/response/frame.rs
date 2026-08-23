//! What a server's response frame carries for a laboratory run.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use super::{Disconnected, Id};
use crate::shared::error::Error;

/// A run's answer: the container's id, its filesystem, and who has
/// left it, for as long as the scope lives.
///
/// # The tag byte
///
/// A payload leads with one byte saying which variant it is — `0` for
/// [`Id`](Self::Id), `1` for [`Filetree`](Self::Filetree), `2` for
/// [`Disconnected`](Self::Disconnected) — and the rest is that
/// variant's own bytes. The same arrangement every tagged frame in
/// this crate uses, and for the same reason: a frame that means
/// something only in the
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
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The container's id. Tag `0`.
    ///
    /// Arrives whenever the provider has it, which is not necessarily
    /// before the filesystem starts reporting. See [`Id`].
    Id(Id),
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
    /// A connector that was attached is not any more. Tag `2`.
    ///
    /// See [`Disconnected`] for the name it carries and what a runner
    /// may assume about it.
    ///
    /// # An event, not a count
    ///
    /// It says one connector left, not how many remain. A runner that
    /// wants a number keeps one: it answered every authorization, so
    /// it saw every arrival, and this is every departure.
    ///
    /// That is the whole reason the nickname exists. A count can be
    /// re-sent and read afresh; a departure cannot, because "one of
    /// them left" is not an answer to "which". A runner holding four
    /// authorized connectors needs the name it chose for them.
    ///
    /// # What the authorize channel is for
    ///
    /// A connector arriving is what prompts
    /// [`Authorize`](super::super::channel_request::Frame::Authorize).
    /// The provider asks the runner whether this one may attach, the
    /// runner answers yes or no and names it, and this is the other
    /// end of that: the same name coming back when it leaves.
    ///
    /// So the two are halves of one exchange, and the order between
    /// them is the only order in this stream that means anything —
    /// nothing can disconnect that was not authorized first.
    Disconnected(Disconnected),
    /// A failure. Tag `3`.
    ///
    /// The laboratory is not running and will not be — the image
    /// would not pull, the container would not start, whatever the
    /// provider knows. It is the one variant that ends the scope
    /// rather than adding to it.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Id`].
const ID: u8 = 0;

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 1;

/// Tag for [`Frame::Disconnected`].
const DISCONNECTED: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// Four variants, four encodings, and none converted into another's
/// to make them match. An id is a string and goes out as its own
/// bytes; a count is four big-endian bytes, the same way `scope` and
/// `channel` are written in every header; a filetree frame is
/// postcard's and is handed to postcard; an error is JSON, because a
/// [`serde_json::Value`] deserializes through `deserialize_any` and
/// cannot come back out of postcard at all.
impl Encode for Frame {
    /// One failure per half that has one, and they are different
    /// libraries'. A string's bytes and a fixed-width integer have no
    /// failure mode.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Id(id) => {
                out.extend_from_slice(&[ID]);
                serde_json::to_writer(out, id).map_err(FrameEncodeError::Id)
            }
            Frame::Filetree(frame) => {
                out.extend_from_slice(&[FILETREE]);
                frame.encode(out).map_err(FrameEncodeError::Filetree)
            }
            Frame::Disconnected(disconnected) => {
                out.extend_from_slice(&[DISCONNECTED]);
                serde_json::to_writer(out, disconnected)
                    .map_err(FrameEncodeError::Disconnected)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(FrameEncodeError::Error)
            }
        }
    }
}

/// A laboratory run response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The id did not serialize.
    Id(serde_json::Error),
    /// The filetree frame did not serialize.
    Filetree(postcard::Error),
    /// The disconnection did not serialize.
    Disconnected(serde_json::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Id(error) => {
                write!(f, "container id did not serialize: {error}")
            }
            FrameEncodeError::Filetree(error) => {
                write!(f, "filetree frame did not serialize: {error}")
            }
            FrameEncodeError::Disconnected(error) => {
                write!(f, "disconnection did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "laboratory run error did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Id(error) => Some(error),
            FrameEncodeError::Filetree(error) => Some(error),
            FrameEncodeError::Disconnected(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

impl Decode<'_> for Frame {
    /// Six ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ID => serde_json::from_slice(rest)
                .map(Frame::Id)
                .map_err(FrameError::Id),
            FILETREE => crate::shared::filetree::response::Frame::decode(rest)
                .map(Frame::Filetree)
                .map_err(FrameError::Filetree),
            DISCONNECTED => serde_json::from_slice(rest)
                .map(Frame::Disconnected)
                .map_err(FrameError::Disconnected),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A laboratory run response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The id did not parse.
    Id(serde_json::Error),
    /// The filetree frame did not decode.
    Filetree(postcard::Error),
    /// The disconnection did not parse.
    Disconnected(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
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
                write!(f, "container id did not parse: {error}")
            }
            FrameError::Filetree(error) => {
                write!(f, "filetree frame did not decode: {error}")
            }
            FrameError::Disconnected(error) => {
                write!(f, "disconnection did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "laboratory run error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Id(error) => Some(error),
            FrameError::Filetree(error) => Some(error),
            FrameError::Disconnected(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
