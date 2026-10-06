//! What a server's response frame carries for a logs read.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use super::ItemWrapper;

/// A logs read's answer: one item, forbidden, or a failure.
///
/// A read is a stream: zero or more items, each one that matches as the
/// log holds it, in the log's order oldest first, then the finish; or
/// exactly one [`Forbidden`](Self::Forbidden), then the finish; or
/// items and then exactly one error, then the finish. A count on the
/// request caps the items, watching or not. Watching, the items go on
/// as they land, and the finish comes when the count is met, when the
/// filter can never match again, on a cancel, or on the agent's
/// deletion — the request frame says exactly when. A payload leads with
/// one byte saying which — `0` for [`Item`](Self::Item), `1` for
/// [`Forbidden`](Self::Forbidden), `2` for [`Error`](Self::Error) — and
/// the rest is that variant's own JSON: one
/// [`ItemWrapper`](super::ItemWrapper) for the first, nothing for the
/// second, the error for the third.
///
/// # A finish with nothing is an answer
///
/// Nothing matched, or the log holds nothing at all: a scope that
/// finishes with no response before it is that answer, not a failure.
/// An [`Error`](Self::Error) is a failure: no agent of the account's is
/// the one named, or the daemon could not read, in its own words. Items
/// sent before the failure precede the error; none follow it.
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
    /// One matching entry of the log: its index, its time, and the
    /// item. Tag `0`.
    Item(ItemWrapper),
    /// The account the request is served for holds no grant allowing
    /// it; nothing was sent. Tag `1`.
    Forbidden,
    /// A failure. Tag `2`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Item`].
const ITEM: u8 = 0;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answer cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Item(item) => {
                out.extend_from_slice(&[ITEM]);
                serde_json::to_writer(out, item)
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
            ITEM => serde_json::from_slice(rest).map(Frame::Item).map_err(FrameError::Item),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A logs read response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The item did not parse.
    Item(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents logs response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents logs response frame tag {tag}"),
            FrameError::Item(error) => write!(f, "agents logs item did not parse: {error}"),
            FrameError::Error(error) => write!(f, "agents logs error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Item(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
