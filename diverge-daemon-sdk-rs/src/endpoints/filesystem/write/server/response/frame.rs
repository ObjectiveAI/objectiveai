//! What a server's response frame carries for a filesystem write.

use std::fmt;

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::shared::containers::write_path;
use diverge_provider_sdk::shared::error::Error;

/// The file landed, or it did not.
///
/// A payload leads with one byte saying which — `0` for
/// [`Written`](Self::Written), `1` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes.
///
/// # How it ends
///
/// | the scope ends with | means |
/// |---------------------|-------|
/// | a [`Written`](Self::Written), then a finish | the file is at the path |
/// | an [`Error`](Self::Error), then a finish | it is not, and nothing partial is |
///
/// Only a finish ends it, and it comes only once the content channel
/// has — or at once, for a write refused before the content was asked
/// for: a path that is not absolute, a parent that is not a
/// directory, a destination that is one.
///
/// # The failure is this scope's, the answer is not
///
/// [`Written`](Self::Written) carries
/// [`write_path::response::Frame`](diverge_provider_sdk::shared::containers::write_path::response::Frame),
/// which means the same thing wherever the exchange happens. The
/// [`Error`](Self::Error) beside it is the exchange's own.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The file is at the path. Tag `0`.
    Written(write_path::response::Frame),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](diverge_provider_sdk::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Written`].
const WRITTEN: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from the half that has one. The
    /// other cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Written(inner) => {
                out.extend_from_slice(&[WRITTEN]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                inner.encode(out).map_err(|error| match error {})
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            WRITTEN => write_path::response::Frame::decode(rest)
                .map(Frame::Written)
                .map_err(|error| match error {}),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A filesystem write response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("filesystem write response frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown filesystem write response frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "filesystem write error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
