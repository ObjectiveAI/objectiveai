//! What a client's request frame carries for a send.

use std::fmt;

use crate::shared::Identity;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Send one message to one identity.
///
/// The tag, the receiver's identity as postcard — its `issuer`, then
/// its `subject`, each a varint byte length and that many bytes of
/// UTF-8 — and then the body, running to the end of the payload. There
/// is no length before the body, because the payload already ends where
/// the body does. The body is borrowed from the payload it is read from,
/// so a provider that keeps the payload keeps the message without a copy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// The receiver: the identity whose listen scope the message is for.
    pub to: Identity,
    /// The body. The provider neither reads nor changes it, and
    /// prescribes no format for it.
    pub body: &'a [u8],
}

/// This frame's tag among the scope-opening requests. See the table
/// in [`endpoints`](crate::provider::endpoints).
const TAG: u8 = 27;

impl Encode for Frame<'_> {
    /// The identity did not serialize.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        postcard::to_io(&self.to, &mut *out)?;
        out.extend_from_slice(self.body);
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        let (to, body) = postcard::take_from_bytes(rest).map_err(FrameError::Identity)?;
        Ok(Frame { to, body })
    }
}

/// A send request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
    /// The receiver's identity did not parse.
    Identity(postcard::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("send request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "send request frame has tag {tag}, expected {TAG}"),
            FrameError::Identity(error) => write!(f, "send receiver identity did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Identity(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
