//! What a server's response frame carries for an image has.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Held, or not found, or the word that the provider has no source to
/// look in, or that it could not.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for [`Held`](Self::Held),
/// `1` for [`NotFound`](Self::NotFound), `2` for
/// [`Unsupported`](Self::Unsupported), `3` for [`Error`](Self::Error)
/// — and the rest is the error's JSON for `Error`, and nothing for
/// the other three.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Held`](Self::Held), then a finish | the provider can supply the image, from the caller's Store or from a registry it is configured with |
/// | [`NotFound`](Self::NotFound), then a finish | it cannot supply the image from either |
/// | [`Unsupported`](Self::Unsupported), then a finish | the provider has neither a Store nor any registry |
/// | an [`Error`](Self::Error), then a finish | it could not look, and nothing is determined |
///
/// # An error is not a not found
///
/// [`NotFound`](Self::NotFound) is an ANSWER: the provider looked and
/// cannot supply the image. It says nothing about why, on purpose —
/// distinguishing "I do not have it" from "I will not serve it to you"
/// would tell an unauthorized caller that a private image exists. An
/// [`Error`](Self::Error) is the absence of an answer, and a caller
/// that treats one as the other either gives up on an image it could
/// have had or retries forever against one it never will.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The provider can supply the image. Tag `0`.
    Held,
    /// It cannot: neither the caller's Store nor a registry it is
    /// configured with, named by one of the image's references, has
    /// it. Tag `1`.
    NotFound,
    /// The provider has neither a Store — its `images_store` is
    /// `false` — nor any registry — its `images_registries` is empty;
    /// see [`info`](crate::provider::endpoints::info). Tag `2`.
    ///
    /// Carries nothing. A caller asks another provider; nothing about
    /// the request is the cause.
    Unsupported,
    /// A failure. Tag `3`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Held`].
const HELD: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, then the error's JSON, or nothing.
impl Encode for Frame {
    /// The ordinary JSON failure, from the one variant that has one.
    /// The tag cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Held => {
                out.extend_from_slice(&[HELD]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
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
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            HELD => Ok(Frame::Held),
            NOT_FOUND => Ok(Frame::NotFound),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An image has result that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("image has result frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image has result frame tag {tag}"),
            FrameError::Error(error) => write!(f, "image has error did not parse: {error}"),
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
