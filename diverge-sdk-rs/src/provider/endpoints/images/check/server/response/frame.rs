//! What a server's response frame carries in an image check.

use std::fmt;

use super::Response;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The payload of a [`ServerFrame::Response`](crate::wire::frame::server::ServerFrame::Response)
/// on channel `0` of an image check.
///
/// A check is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. There are three
/// things that reply can be: an answer, the news that the question
/// could not be answered, or the word that the question named no
/// source to look in.
///
/// A payload leads with one byte saying which — `0` for
/// [`Response`](Self::Response), `1` for [`Error`](Self::Error), `2`
/// for [`NoSource`](Self::NoSource) — and the rest is that variant's
/// own JSON; `NoSource` carries nothing.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | a [`Response`](Self::Response), then a finish | the provider looked where it was asked to, and this is what it found |
/// | an [`Error`](Self::Error), then a finish | the provider could not look, and nothing is determined |
/// | [`NoSource`](Self::NoSource), then a finish | the request asked the provider to look nowhere, and nothing is determined |
///
/// # The frame type is still not a wire shape
///
/// It carries no serde derives, and it is not what gets serialized —
/// it is the DISPATCH layer, saying which payload a response frame
/// holds. What changed is that the choice is now written down: it used
/// to be settled entirely by the scope the frame arrived in, and one
/// byte at the front now settles it instead.
///
/// That byte is not decoration. [`Response`](super::Response) is
/// untagged and tells its two answers apart by a `type` constant
/// inside each one, while an [`Error`] is an arbitrary JSON value —
/// including, legitimately, an object with a `type` field. Leaving the
/// two to be distinguished by their JSON would mean a provider's error
/// text could be read as an answer, and "I could not tell you" would
/// arrive looking like "no".
///
/// # An error is not an unavailable
///
/// They are the two things this scope can end with and they mean
/// opposite things about the image.
///
/// [`Unavailable`](super::Unavailable) is an ANSWER: the provider
/// looked and will not supply it. It says nothing about why, on
/// purpose — distinguishing "I do not have it" from "I will not serve
/// it to you" would tell an unauthorized caller that a private image
/// exists.
///
/// An [`Error`](Self::Error) is the absence of an answer. Nothing was
/// determined about the image, and a caller that treats one as the
/// other either gives up on an image it could have had or retries
/// forever against one it never will.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The answer to the check. Tag `0`.
    Response(Response),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
    /// The request asked for no source, and nothing was looked at.
    /// Tag `2`.
    ///
    /// Neither an answer nor a failure: the question was empty. A
    /// caller that wanted an answer asks again with at least one of
    /// the two sources set.
    NoSource,
}

/// Tag for [`Frame::Response`].
const RESPONSE: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Tag for [`Frame::NoSource`].
const NO_SOURCE: u8 = 2;

/// A tag, then the variant's own JSON. The newtype around an answer
/// still leaves no trace, and neither does the untagged enum inside
/// it — what a response encodes to is exactly what it encoded to
/// before the tag existed.
impl Encode for Frame {
    /// The ordinary JSON failure, from whichever half is present.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Response(response) => {
                out.extend_from_slice(&[RESPONSE]);
                serde_json::to_writer(out, response)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
            Frame::NoSource => {
                out.extend_from_slice(&[NO_SOURCE]);
                Ok(())
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
            RESPONSE => serde_json::from_slice(rest)
                .map(Frame::Response)
                .map_err(FrameError::Response),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            NO_SOURCE => Ok(Frame::NoSource),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An image check response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The answer did not parse.
    ///
    /// [`Response`](super::Response) is untagged, so a payload that is
    /// neither an available nor an unavailable answer fails here
    /// rather than decoding as something half-right.
    Response(serde_json::Error),
    /// The error did not parse.
    ///
    /// Which is its own small joke and its own real problem: a
    /// provider whose failure report is malformed has told a caller
    /// that something went wrong and nothing else.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("image check response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown image check response frame tag {tag}")
            }
            FrameError::Response(error) => {
                write!(f, "image check answer did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "image check error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Response(error) | FrameError::Error(error) => {
                Some(error)
            }
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
