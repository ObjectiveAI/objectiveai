//! What a server's response frame carries for a list_for.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use crate::provider::endpoints::containers::tools::list_for::server::response::Container;

/// A list_for's answer: one container, no such provider, or a failure.
///
/// A list_for is a stream: zero or more containers, each as the
/// provider sent it and in the order it did, then the finish when the
/// provider's scope finishes; or exactly one
/// [`NoProvider`](Self::NoProvider), then the finish; or containers and
/// then exactly one error, then the finish. A payload leads with one
/// byte saying which — `0` for [`Container`](Self::Container), `1` for
/// [`NoProvider`](Self::NoProvider), `2` for [`Error`](Self::Error) —
/// and the rest is that variant's own JSON: the provider protocol's
/// [`Container`] for the first, nothing for the second, the error for
/// the third.
///
/// # A finish with nothing is an answer
///
/// The identity runs no tool container there, or no runner let the
/// caller see one: a scope that finishes with no response before it is
/// that answer, not a failure, and the two are not told apart.
/// [`NoProvider`](Self::NoProvider) is an ANSWER: no provider of the
/// caller's is the one named, and nothing was asked. An
/// [`Error`](Self::Error) is a failure: the daemon could not ask, or
/// the provider answered an error, in whichever's words. Containers
/// sent before the failure precede the error; none follow it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One tool container the identity runs, as the provider told of
    /// it. Tag `0`.
    Container(Container),
    /// No provider of the caller's is the one named; nothing was asked.
    /// Tag `1`.
    NoProvider,
    /// A failure. Tag `2`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Container`].
const CONTAINER: u8 = 0;

/// Tag for [`Frame::NoProvider`].
const NO_PROVIDER: u8 = 1;

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
            Frame::Container(container) => {
                out.extend_from_slice(&[CONTAINER]);
                serde_json::to_writer(out, container)
            }
            Frame::NoProvider => {
                out.extend_from_slice(&[NO_PROVIDER]);
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
            CONTAINER => serde_json::from_slice(rest).map(Frame::Container).map_err(FrameError::Container),
            NO_PROVIDER => Ok(Frame::NoProvider),
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
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The container did not parse.
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
