//! What a client's channel request frame carries in a loop.

use std::error;
use std::fmt;

use super::Enqueue;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The payload of a client's
/// [`ChannelRequest`](crate::frame::client::ClientFrame::ChannelRequest)
/// on this endpoint.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own JSON.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Enqueue`](Self::Enqueue) |
///
/// # A tag with one variant, and why it is spent anyway
///
/// This crate's habit is not to spend a discriminant on nothing to
/// discriminate — the server's own channel request went years as a
/// struct for exactly that reason. The byte is spent here because the
/// second variant is already designed: withdrawing an enqueued
/// message is a request of its own, and retrofitting a tag under a
/// deployed tagless frame is a breaking change this endpoint has paid
/// for once already.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A message for the running conversation's queue. Tag `0`.
    ///
    /// Answered once — by an
    /// [`enqueue::Frame`](crate::endpoints::agentic_loop::run::server::channel_response::enqueue::Frame)
    /// naming the message's fate — and then the finish.
    Enqueue(Enqueue),
}

/// Tag for [`Frame::Enqueue`].
const ENQUEUE: u8 = 0;

/// A tag, then that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. Every variant is serialized, and the
    /// tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Enqueue(request) => {
                out.extend_from_slice(&[ENQUEUE]);
                request.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ENQUEUE => Enqueue::decode(rest)
                .map(Frame::Enqueue)
                .map_err(FrameError::Body),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is not this frame's one.
    ///
    /// What a client newer than its provider produces, which is the
    /// case the tag exists to make survivable: a reader that does not
    /// know a variant says so, rather than reading somebody else's
    /// bytes as its own.
    UnknownTag(u8),
    /// The payload after the tag did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown channel request tag {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "channel request did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
