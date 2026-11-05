//! What a server's response frame carries for a discover listing.

use std::fmt;

use super::Discoverable;
use crate::shared::listing::Change;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A discoverable connected identity and its description, as it comes
/// to be listed or goes; or the word that the listing is whole.
///
/// A listing is a stream kept open. A payload leads with one byte
/// saying which — `0` for [`Added`](Self::Added), `1` for
/// [`Removed`](Self::Removed), `2` for [`Listed`](Self::Listed) — and
/// the rest is that variant's own bytes: the [`Discoverable`] as
/// postcard, or nothing. The rules:
///
/// 1. **The snapshot.** First every identity connected to the provider
///    whose discover flag was `1` or `2`, each as `Added` with the
///    description its flag `2` carried — whichever end dialled, the
///    asking identity among them when its own flag was `1` or `2`.
/// 2. **Listed, once.** Then exactly one `Listed`, before any other
///    response.
/// 3. **Kept.** Then, for the scope's life, an identity whose
///    connection opens with the flag `1` or `2` as `Added`, and one
///    whose connection ends as `Removed`, as it was added.
/// 4. **Once per identity.** An identity holds at most one connection
///    to a provider, so it is sent as added once while it is listed,
///    and as removed only after it was added. An identity that
///    disconnects and connects again is sent `Removed`, then `Added`.
///    Nothing is told as changed: a description is its connection's,
///    and does not change while it is listed.
/// 5. **No end of its own.** The finish follows the caller's stop or
///    the caller's connection ending, and nothing else: a provider with
///    nothing discoverable sends `Listed` at once, and is watched.
/// 6. **No error.** A provider always knows what is connected to it,
///    and every provider serves this listing, so no response refuses.
///
/// # A finish with nothing is an answer
///
/// The caller stopped before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// A discoverable identity, listed. Tag `0`.
    ///
    /// Nothing promises an order.
    Added(Discoverable),
    /// An identity listed already, as it was added, whose connection
    /// ended. Tag `1`.
    Removed(Discoverable),
    /// Every identity discoverable when the scope opened has been
    /// listed. Tag `2`.
    ///
    /// Carries nothing — the variant is bare. A caller that wants the
    /// listing once stops here; one that watches reads on.
    Listed,
}

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Removed`].
const REMOVED: u8 = 1;

/// Tag for [`Frame::Listed`].
const LISTED: u8 = 2;

/// The tag, then the [`Discoverable`] as postcard, as a listing's item
/// is.
impl Encode for Frame {
    /// The identity did not serialize.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Added(discoverable) => {
                out.extend_from_slice(&[ADDED]);
                postcard::to_io(discoverable, &mut *out).map(|_| ()).map_err(FrameEncodeError::Identity)
            }
            Frame::Removed(discoverable) => {
                out.extend_from_slice(&[REMOVED]);
                postcard::to_io(discoverable, &mut *out).map(|_| ()).map_err(FrameEncodeError::Identity)
            }
            Frame::Listed => {
                out.extend_from_slice(&[LISTED]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail.
    type Error = FrameDecodeError;

    fn decode(bytes: &[u8]) -> Result<Self, FrameDecodeError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameDecodeError::Empty)?;
        match *tag {
            ADDED => postcard::from_bytes(rest).map(Frame::Added).map_err(FrameDecodeError::Identity),
            REMOVED => postcard::from_bytes(rest).map(Frame::Removed).map_err(FrameDecodeError::Identity),
            LISTED => Ok(Frame::Listed),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

impl Frame {
    /// What this frame tells a listing — a [`Change`], for
    /// [`Listing::push`](crate::shared::listing::Listing::push). Every
    /// frame is one, since no response ends the scope; none is
    /// `Changed`.
    pub fn change(self) -> Change<Discoverable> {
        match self {
            Frame::Added(discoverable) => Change::Added(discoverable),
            Frame::Removed(discoverable) => Change::Removed(discoverable),
            Frame::Listed => Change::Listed,
        }
    }
}

/// A discover listing response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The identity did not serialize.
    Identity(postcard::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Identity(error) => write!(f, "discover listing identity did not serialize: {error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Identity(error) => Some(error),
        }
    }
}

/// A discover listing response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The identity did not parse.
    Identity(postcard::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("discover listing response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => write!(f, "unknown discover listing response frame tag {tag}"),
            FrameDecodeError::Identity(error) => write!(f, "discover listing identity did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Identity(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
