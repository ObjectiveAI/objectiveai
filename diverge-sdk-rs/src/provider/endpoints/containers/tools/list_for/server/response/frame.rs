//! What a server's response frame carries for a listing.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use super::Container;

/// A listing's answer: a container added, a container removed, the
/// word that the listing is whole, or a failure.
///
/// A listing is a stream kept open. Zero or more containers are
/// added — one per tool container the identity runs whose runner
/// said yes, each sent the moment that runner answered and in no
/// order a lister may rely on — among them, once every container
/// running when the scope opened has had its runner answer or go,
/// exactly one [`Listed`](Self::Listed); after it, a container the
/// identity starts is added as its runner allows, and a container
/// whose run ends is removed, only ever after it was added. The
/// finish follows the lister's stop, the lister's connection ending,
/// or exactly one [`Error`](Self::Error), and nothing else: the
/// identity running no container is not an end. A payload leads with
/// one byte saying which — `0` for [`Added`](Self::Added), `1` for
/// [`Removed`](Self::Removed), `2` for [`Listed`](Self::Listed), `3`
/// for [`Error`](Self::Error) — and the rest, for the first two and
/// the last, is that variant's own JSON; `Listed` carries nothing.
///
/// # A finish with nothing is an answer
///
/// The lister stopped before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure, and the two cases are not told apart. An
/// [`Error`](Self::Error) is a failure: the provider could not ask,
/// for whatever reason it knows. Responses sent before the failure
/// precede the error; none follow it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One container, its runner having said yes. Tag `0`.
    Added(Container),
    /// One container, added earlier, whose run ended. Tag `1`.
    Removed(Container),
    /// Every container running when the scope opened has been asked
    /// about and answered for, or its runner has gone: what was
    /// added before this is the listing as it stood. Tag `2`.
    ///
    /// Carries nothing — the variant is bare. A lister that wants
    /// the listing once stops here; one that watches reads on.
    Listed,
    /// A failure. Tag `3`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Removed`].
const REMOVED: u8 = 1;

/// Tag for [`Frame::Listed`].
const LISTED: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, then the variant's own JSON, or the tag alone.
impl Encode for Frame {
    /// The ordinary JSON failure, from any half that has one.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Added(container) => {
                out.extend_from_slice(&[ADDED]);
                serde_json::to_writer(out, container)
            }
            Frame::Removed(container) => {
                out.extend_from_slice(&[REMOVED]);
                serde_json::to_writer(out, container)
            }
            Frame::Listed => {
                out.extend_from_slice(&[LISTED]);
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
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ADDED => serde_json::from_slice(rest).map(Frame::Added).map_err(FrameError::Container),
            REMOVED => serde_json::from_slice(rest).map(Frame::Removed).map_err(FrameError::Container),
            LISTED => Ok(Frame::Listed),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A listing response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The container, added or removed, did not parse.
    Container(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools list_for response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools list_for response frame tag {tag}"),
            FrameError::Container(error) => write!(f, "tools list_for container did not parse: {error}"),
            FrameError::Error(error) => write!(f, "tools list_for error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Container(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
