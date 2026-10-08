//! What a server's response frame carries for list.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use super::Incoming;

/// A list's answer: a credential added, changed or removed, the word that
/// the list is whole, forbidden, or a failure.
///
/// A list is a stream kept open. First every credential that matches as
/// the daemon holds it, oldest first, the first `count` of them,
/// each as [`Added`](Self::Added); then exactly one
/// [`Listed`](Self::Listed); then, for the scope's life, a credential come
/// to match as `Added`, one that matches still and differs from what
/// was last sent as [`Changed`](Self::Changed), one that matches no
/// more as [`Removed`](Self::Removed) — the listing kept as the first
/// `count` that match, so one leaving that window is removed and one
/// entering it added. The finish follows the client's cancel, the
/// client's connection ending, or exactly one [`Error`](Self::Error);
/// or the scope is exactly one [`Forbidden`](Self::Forbidden), then
/// the finish. A payload leads with one byte saying which — `0` for
/// `Added`, `1` for `Changed`, `2` for `Removed`, `3` for `Listed`,
/// `4` for `Forbidden`, `5` for `Error` — and the rest is that
/// variant's own JSON: one [`Incoming`](super::Incoming) for the first three,
/// the error for the last, nothing for the others.
///
/// # A finish with nothing is an answer
///
/// The client cancelled before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure; a listing with nothing that matches is
/// `Listed` at once, and watched. An [`Error`](Self::Error) is a
/// failure: the daemon could not list, for whatever reason it knows.
/// What was sent before the failure precedes the error; nothing
/// follows it.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's, or the
/// `account` of the container it came from — holds no grant allowing
/// what the request asks over what it names, and nothing changed. Where
/// the answer is a stream it comes as an error does: exactly one, with
/// nothing before it, then the finish. Nothing is retried. See
/// [`grant`](crate::daemon::grant).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One credential that matches, as the daemon holds it, listed now and
    /// not before. Tag `0`.
    Added(Incoming),
    /// One credential listed already, as it is now, differing from what was
    /// last sent. Tag `1`.
    Changed(Incoming),
    /// One credential listed already, matching no more: as it was last
    /// sent. Tag `2`.
    Removed(Incoming),
    /// Every credential that matched when the scope opened has been sent.
    /// Tag `3`.
    ///
    /// Carries nothing — the variant is bare. A client that wants the
    /// list once cancels here; one that watches reads on.
    Listed,
    /// The account the request is served for holds no grant allowing
    /// it; nothing was sent. Tag `4`.
    Forbidden,
    /// A failure. Tag `5`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Changed`].
const CHANGED: u8 = 1;

/// Tag for [`Frame::Removed`].
const REMOVED: u8 = 2;

/// Tag for [`Frame::Listed`].
const LISTED: u8 = 3;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 4;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 5;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Added(item) => {
                out.extend_from_slice(&[ADDED]);
                serde_json::to_writer(out, item)
            }
            Frame::Changed(item) => {
                out.extend_from_slice(&[CHANGED]);
                serde_json::to_writer(out, item)
            }
            Frame::Removed(item) => {
                out.extend_from_slice(&[REMOVED]);
                serde_json::to_writer(out, item)
            }
            Frame::Listed => {
                out.extend_from_slice(&[LISTED]);
                Ok(())
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
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
            ADDED => serde_json::from_slice(rest).map(Frame::Added).map_err(FrameError::Incoming),
            CHANGED => serde_json::from_slice(rest).map(Frame::Changed).map_err(FrameError::Incoming),
            REMOVED => serde_json::from_slice(rest).map(Frame::Removed).map_err(FrameError::Incoming),
            LISTED => Ok(Frame::Listed),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A list response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// The credential, added, changed or removed, did not parse.
    Incoming(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("providers incoming list response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown providers incoming list response frame tag {tag}"),
            FrameError::Incoming(error) => write!(f, "providers incoming list credential did not parse: {error}"),
            FrameError::Error(error) => write!(f, "providers incoming list error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Incoming(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
