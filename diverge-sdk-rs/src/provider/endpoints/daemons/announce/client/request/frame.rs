//! What a client's request frame carries for an announce.

use std::str::{self, Utf8Error};

use crate::shared::description::{Description, DescriptionError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask a provider to list the caller's identity in its
/// [`daemons::list`](crate::provider::endpoints::daemons::list), with
/// this announcement, for as long as the scope is open.
///
/// The identity announced is the one the connection was authorized
/// under, and nothing else names it.
///
/// # The layout
///
/// 1. **The tag.** The first byte, `29`.
/// 2. **The announcement.** Every byte after the tag: the
///    [`Description`] as UTF-8, and nothing else — no length, since the
///    payload already ends where the announcement does.
/// 3. **Malformed.** No bytes after the tag, bytes that are not UTF-8,
///    and text that breaks a rule of `Description` — one line of plain
///    text, 1 to 132 characters, trimmed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame {
    /// The text the caller is listed with.
    pub announcement: Description,
}

/// This frame's tag among the scope-opening requests. See the table
/// in [`endpoints`](crate::provider::endpoints).
const TAG: u8 = 29;

/// The tag, then the announcement's own bytes.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): a known byte, and a
    /// text's bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        out.extend_from_slice(self.announcement.as_str().as_bytes());
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        let text = str::from_utf8(rest).map_err(FrameError::Utf8)?;
        let announcement = Description::new(text).map_err(FrameError::Announcement)?;
        Ok(Frame { announcement })
    }
}

/// An announce request that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
    /// The announcement was not UTF-8.
    Utf8(Utf8Error),
    /// The announcement broke a rule of [`Description`]: empty among
    /// them.
    Announcement(DescriptionError),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("announce request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "announce request frame has tag {tag}, expected {TAG}"),
            FrameError::Utf8(error) => write!(f, "announce request's announcement is not utf-8: {error}"),
            FrameError::Announcement(error) => write!(f, "announce request's announcement is not one: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Utf8(error) => Some(error),
            FrameError::Announcement(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
