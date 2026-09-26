//! What a server's channel response frame carries on a transfer channel.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::transfer;
use crate::shared::error::Error;

/// The file landed in the other container, or it did not.
///
/// A payload leads with one byte saying which — `0` for
/// [`Transferred`](Self::Transferred), `1` for [`Error`](Self::Error)
/// — and the rest is that variant's own bytes.
///
/// # How it ends
///
/// | the channel ends with | means |
/// |-----------------------|-------|
/// | a [`Transferred`](Self::Transferred), then a finish | the file is at the destination |
/// | an [`Error`](Self::Error), then a finish | it is not, and nothing partial is |
///
/// Only a finish ends it. What a caller does about an error is not
/// specified here.
///
/// # The failure is this scope's, the answer is not
///
/// [`Transferred`](Self::Transferred) carries
/// [`transfer::response::Frame`](crate::shared::containers::transfer::response::Frame),
/// which means the same thing wherever the exchange happens. The
/// [`Error`](Self::Error) beside it is the exchange's own.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The file is at the destination. Tag `0`.
    Transferred(transfer::response::Frame),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Transferred`].
const TRANSFERRED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from the half that has one. The
    /// other cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Transferred(inner) => {
                out.extend_from_slice(&[TRANSFERRED]);
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
            TRANSFERRED => transfer::response::Frame::decode(rest)
                .map(Frame::Transferred)
                .map_err(|error| match error {}),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A transfer response frame that could not be read.
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
            FrameError::Empty => f.write_str("transfer response frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown transfer response frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "transfer error did not parse: {error}")
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
