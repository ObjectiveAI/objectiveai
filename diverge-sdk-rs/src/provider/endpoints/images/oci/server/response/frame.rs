//! What a server's response frame carries on a pull's main stream.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The one thing a provider says on channel `0` of a pull, and only
/// when it will not serve one: it cannot supply the image, or it has
/// no source to supply any from.
///
/// An ordinary pull sends nothing on channel `0` — every answer comes
/// on an ask's own channel, and the finish is the scope's end. A
/// provider that will not serve the pull for one of these two reasons
/// sends exactly one of these first, and the finish follows it; no ask
/// is answered. A payload is one byte, `0` for
/// [`NotFound`](Self::NotFound) and `1` for
/// [`Unsupported`](Self::Unsupported), and nothing after it.
///
/// | the scope | means |
/// |-----------|-------|
/// | nothing, and stays open | the provider serves the pull, and every ask the caller opens is answered on its channel |
/// | nothing, then a finish | the caller cancelled, or the provider would not serve the pull for a reason it does not say |
/// | a not found, then a finish | the provider cannot supply the image, from the caller's Store or from a registry it is configured with |
/// | an unsupported, then a finish | the provider has neither a Store nor any registry; nothing was asked and nothing is held |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// The provider cannot supply the image — neither the caller's
    /// Store nor a registry it is configured with, named by one of the
    /// image's references, has it — and will not serve the pull. Tag
    /// `0`.
    ///
    /// Carries nothing, and says nothing about why, as
    /// [`has`](crate::provider::endpoints::images::has) says nothing.
    NotFound,
    /// The provider has neither a Store — its `images_store` is
    /// `false` — nor any registry — its `images_registries` is empty;
    /// see [`info`](crate::provider::endpoints::info) — and will not
    /// serve the pull. Tag `1`.
    ///
    /// Carries nothing. A caller asks another provider; nothing about
    /// the request is the cause.
    Unsupported,
}

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 0;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 1;

/// The tag, and nothing after it.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): a known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, _) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            NOT_FOUND => Ok(Frame::NotFound),
            UNSUPPORTED => Ok(Frame::Unsupported),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A pull response frame that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("image pull response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image pull response frame tag {tag}"),
        }
    }
}

impl std::error::Error for FrameError {}
