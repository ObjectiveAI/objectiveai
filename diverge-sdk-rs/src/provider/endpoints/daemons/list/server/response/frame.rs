//! What a server's response frame carries for a daemons list.

use std::fmt;

use super::Listed;
use crate::shared::Identity;
use crate::shared::listing::Change;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// An announced connected identity and its announcement, as it comes
/// to be listed or goes; the word that the listing is whole; the word
/// that one listed came to listen for messages, or stopped; or the
/// refusal of a second listing on one connection.
///
/// A listing is a stream kept open. A payload leads with one byte
/// saying which — `0` for [`Added`](Self::Added), `1` for
/// [`Removed`](Self::Removed), `2` for [`Listed`](Self::Listed), `3`
/// for [`Listening`](Self::Listening), `4` for
/// [`NotListening`](Self::NotListening), `5` for
/// [`AlreadyOpen`](Self::AlreadyOpen) — and the rest is that variant's
/// own bytes: the [`Listed`] as postcard, the [`Identity`] as
/// postcard, or nothing. The rules:
///
/// 1. **The snapshot.** First every identity connected to the provider
///    that holds a
///    [`daemons::announce`](crate::provider::endpoints::daemons::announce)
///    open, each as `Added` with the announcement that scope carries —
///    whichever end dialled, the asking identity among them when it
///    holds one itself.
/// 2. **Listed, once.** Then exactly one `Listed`, before any other
///    response.
/// 3. **Kept.** Then, for the scope's life, an identity whose announce
///    scope opens as `Added`, and one whose announce scope ends — on
///    its cancel, or with its connection — as `Removed`, as it was
///    added.
/// 4. **Once per identity.** An identity holds at most one connection
///    to a provider, and a connection at most one announce scope, so
///    an identity is sent as added once while it is listed, and as
///    removed only after it was added. An identity that stops
///    announcing and announces again is sent `Removed`, then `Added`.
///    Nothing is told as changed: an announcement is its scope's, and
///    an identity that changes its announcement cancels the scope and
///    opens another, which is `Removed`, then `Added`.
/// 8. **Listening, by the key alone.** An identity listed already whose
///    [`daemons::listen`](crate::provider::endpoints::daemons::listen)
///    opens is sent `Listening`, and one whose listen closes is sent
///    `NotListening`, each carrying the identity and nothing else — the
///    listing's key, never the whole item. Each is sent only when it
///    changes what was last told of the identity, and only after the
///    identity was added. `Removed` carries the item with its
///    `listening` as the events left it.
/// 5. **No end of its own.** The finish follows the caller's cancel or
///    the caller's connection ending, and nothing else: a provider with
///    nothing announced sends `Listed` at once, and is watched.
/// 6. **No error.** A provider always knows what is connected to it,
///    and every provider serves this listing, so no response fails.
/// 7. **One at a time.** A connection with a daemons list open
///    already is sent `AlreadyOpen` first and alone, and the finish
///    follows it — see the
///    [endpoints](crate::provider::endpoints#one-open-at-a-time). It
///    is the one response that refuses.
///
/// # A finish with nothing is an answer
///
/// The caller cancelled before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// An announced identity, listed. Tag `0`.
    ///
    /// Nothing promises an order.
    Added(Listed),
    /// An identity listed already, as it was added, whose announce
    /// scope ended. Tag `1`.
    Removed(Listed),
    /// Every identity announced when the scope opened has been
    /// listed. Tag `2`.
    ///
    /// Carries nothing — the variant is bare. A caller that wants the
    /// listing once stops here; one that watches reads on.
    Listed,
    /// An identity listed already came to listen for messages (rule 8).
    /// Tag `3`.
    Listening(Identity),
    /// An identity listed already stopped listening for messages
    /// (rule 8). Tag `4`.
    NotListening(Identity),
    /// Refused: this connection has a daemons list open already.
    /// Tag `5`.
    ///
    /// Carries nothing. Sent first and alone, and the finish follows
    /// it: nothing was listed (rule 7).
    AlreadyOpen,
}

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Removed`].
const REMOVED: u8 = 1;

/// Tag for [`Frame::Listed`].
const LISTED: u8 = 2;

/// Tag for [`Frame::Listening`].
const LISTENING: u8 = 3;

/// Tag for [`Frame::NotListening`].
const NOT_LISTENING: u8 = 4;

/// Tag for [`Frame::AlreadyOpen`].
const ALREADY_OPEN: u8 = 5;

/// The tag, then the [`Listed`] as postcard, as a listing's item
/// is.
impl Encode for Frame {
    /// The identity did not serialize.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Added(listed) => {
                out.extend_from_slice(&[ADDED]);
                postcard::to_io(listed, &mut *out).map(|_| ()).map_err(FrameEncodeError::Identity)
            }
            Frame::Removed(listed) => {
                out.extend_from_slice(&[REMOVED]);
                postcard::to_io(listed, &mut *out).map(|_| ()).map_err(FrameEncodeError::Identity)
            }
            Frame::Listed => {
                out.extend_from_slice(&[LISTED]);
                Ok(())
            }
            Frame::Listening(identity) => {
                out.extend_from_slice(&[LISTENING]);
                postcard::to_io(identity, &mut *out).map(|_| ()).map_err(FrameEncodeError::Identity)
            }
            Frame::NotListening(identity) => {
                out.extend_from_slice(&[NOT_LISTENING]);
                postcard::to_io(identity, &mut *out).map(|_| ()).map_err(FrameEncodeError::Identity)
            }
            Frame::AlreadyOpen => {
                out.extend_from_slice(&[ALREADY_OPEN]);
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
            LISTENING => postcard::from_bytes(rest).map(Frame::Listening).map_err(FrameDecodeError::Identity),
            NOT_LISTENING => postcard::from_bytes(rest).map(Frame::NotListening).map_err(FrameDecodeError::Identity),
            ALREADY_OPEN => Ok(Frame::AlreadyOpen),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

impl Frame {
    /// What this frame tells a listing — a [`Change`], for
    /// [`Listing::push`](crate::shared::listing::Listing::push) — or
    /// the frame itself where it tells a listing nothing a listing keeps
    /// by the item: a listening change, read by the key, or already
    /// open, the one answer that ends the scope. None is `Changed`.
    pub fn change(self) -> Result<Change<Listed>, Frame> {
        match self {
            Frame::Added(listed) => Ok(Change::Added(listed)),
            Frame::Removed(listed) => Ok(Change::Removed(listed)),
            Frame::Listed => Ok(Change::Listed),
            other => Err(other),
        }
    }
}

/// A daemons list response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The identity did not serialize.
    Identity(postcard::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Identity(error) => write!(f, "daemons list identity did not serialize: {error}"),
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

/// A daemons list response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// The identity did not parse.
    Identity(postcard::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("daemons list response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => write!(f, "unknown daemons list response frame tag {tag}"),
            FrameDecodeError::Identity(error) => write!(f, "daemons list identity did not parse: {error}"),
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
