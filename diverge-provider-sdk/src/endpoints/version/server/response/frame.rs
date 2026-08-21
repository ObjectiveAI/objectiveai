//! What a server's response frame carries for a version request.

use std::fmt;
use std::str::{self, Utf8Error};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// What the provider says it is.
///
/// One of these on channel `0`, then the scope finishes.
///
/// # The layout
///
/// ```text
/// [0][version: utf-8]
/// ```
///
/// The version runs to the end of the payload, so it needs no length.
/// It may be empty, which is a provider declining to say — an answer,
/// and one a caller can act on, rather than the absence of one.
///
/// # It is a string, and this layer does not read it
///
/// No number, no three fields, no ordering. What a version MEANS is
/// between the two ends: a semantic version, a build hash, a date, a
/// name. A shape imposed here would be this specification deciding how
/// providers are allowed to version themselves, which is not its to
/// decide and not something it could revise once decided.
///
/// So comparing two of them is a caller's business. A caller that
/// wants to know whether a provider is new enough knows what its own
/// versions look like; nothing here can help it and nothing here will
/// get in the way.
///
/// # There is no failure
///
/// Alone among the responses in this specification. Every other scope
/// can come back with the provider saying it could not — an image it
/// cannot supply, a container that would not start — because every
/// other scope asks it to DO something.
///
/// This asks it to say what it is, which it always knows. A provider
/// that could not answer this could not have received the question.
///
/// # A struct with a tag, rather than the string alone
///
/// The tag is what leaves room. One kind of answer today is not a
/// promise of one forever, and a payload that was bare version bytes
/// could not grow a second kind without every existing reader
/// misreading it. One byte holds that door open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame<'a>(
    /// The version, borrowed from the frame it arrived in.
    pub &'a str,
);

/// Tag for [`Frame`].
const VERSION: u8 = 0;

/// A tag, then the string's own bytes.
impl Encode for Frame<'_> {
    /// [`Infallible`](std::convert::Infallible): a known byte and a
    /// string's own.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[VERSION]);
        out.extend_from_slice(self.0.as_bytes());
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != VERSION {
            return Err(FrameError::UnknownTag(*tag));
        }
        str::from_utf8(rest).map(Frame).map_err(FrameError::Version)
    }
}

/// A version answer that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is not this frame's one.
    ///
    /// Which is how an answer kind added later arrives at a reader
    /// built before it — as something unreadable rather than as a
    /// version that was never sent.
    UnknownTag(u8),
    /// The version was not UTF-8.
    Version(Utf8Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("version answer frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown version answer tag {tag}")
            }
            FrameError::Version(error) => {
                write!(f, "version was not utf-8: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Version(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
