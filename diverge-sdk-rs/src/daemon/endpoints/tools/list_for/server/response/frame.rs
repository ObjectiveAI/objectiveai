//! What a server's response frame carries for a list_for.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use crate::provider::endpoints::containers::tools::list_for::server::response::Container;

/// A list_for's answer: a container added or removed, the word that
/// the listing is whole, no such provider, forbidden, or a failure.
///
/// A list_for is a stream kept open: every container the provider
/// adds, as it adds it, as [`Added`](Self::Added); the provider's word
/// that the listing is whole as [`Listed`](Self::Listed), exactly
/// once; then, for the scope's life, containers added and removed as
/// the provider sends them. The finish follows the client's cancel,
/// the client's connection ending, the provider's scope ending, or
/// exactly one [`Error`](Self::Error); or the scope is exactly one
/// [`NoProvider`](Self::NoProvider) or [`Forbidden`](Self::Forbidden),
/// then the finish. A payload leads with one byte saying which — `0`
/// for `Added`, `1` for [`Removed`](Self::Removed), `2` for `Listed`,
/// `3` for `NoProvider`, `4` for `Forbidden`, `5` for `Error` — and
/// the rest is that variant's own JSON: the provider protocol's
/// [`Container`] for the first two, the error for the last, nothing
/// for the others.
///
/// # A finish with nothing is an answer
///
/// The client cancelled before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure. [`NoProvider`](Self::NoProvider) is an ANSWER:
/// no provider of the caller's is the one named, and nothing was
/// asked. An [`Error`](Self::Error) is a failure: the daemon could not
/// ask, or the provider answered an error, in whichever's words.
/// Responses sent before the failure precede the error; none follow
/// it.
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
    /// One tool container the identity runs, as the provider told of
    /// it: its runner let the caller see it. Tag `0`.
    Added(Container),
    /// One tool container told of earlier, whose run ended. Tag `1`.
    Removed(Container),
    /// Every container the identity ran when the scope opened has been
    /// told of or will not be: the listing as it stood. Tag `2`.
    ///
    /// Carries nothing — the variant is bare. A client that wants the
    /// listing once cancels here; one that watches reads on.
    Listed,
    /// No provider of the caller's is the one named; nothing was asked.
    /// Tag `3`.
    NoProvider,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `4`.
    Forbidden,
    /// A failure. Tag `5`.
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

/// Tag for [`Frame::NoProvider`].
const NO_PROVIDER: u8 = 3;

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
            Frame::NoProvider => {
                out.extend_from_slice(&[NO_PROVIDER]);
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
            ADDED => serde_json::from_slice(rest).map(Frame::Added).map_err(FrameError::Container),
            REMOVED => serde_json::from_slice(rest).map(Frame::Removed).map_err(FrameError::Container),
            LISTED => Ok(Frame::Listed),
            NO_PROVIDER => Ok(Frame::NoProvider),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A list_for response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
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
