//! What a client's channel request frame carries in a loop.

use std::error;
use std::fmt;

use super::{Dequeue, Enqueue, Postgres};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The payload of a client's
/// [`ChannelRequest`](crate::frame::client::ClientFrame::ChannelRequest)
/// on this endpoint.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes — JSON for the queue verbs, four bytes for a
/// connection id.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Enqueue`](Self::Enqueue) |
/// | `1` | [`Dequeue`](Self::Dequeue) |
/// | `2` | [`Postgres`](Self::Postgres) |
///
/// # Two verbs, one queue
///
/// Everything a client can do to a running conversation is done to
/// its QUEUE: put a message in, or clear what has not yet been
/// taken. Neither touches the turn in flight — the loop itself is
/// the server's to run and the scope's to end.
///
/// # And one that does not reach into the container
///
/// [`Postgres`](Self::Postgres) opens outward for a different reason.
/// It asks for bytes the provider is already holding — what the
/// container wrote on a database connection — so topology has
/// nothing to do with it: the writes have to arrive as a RESPONSE
/// stream, because only a responder can finish a channel and the
/// provider needs to be able to say the container has gone.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A message for the running conversation's queue. Tag `0`.
    ///
    /// Answered once — by an
    /// [`enqueue::Frame`](crate::endpoints::agentic_loop::run::server::channel_response::enqueue::Frame)
    /// naming the message's fate — and then the finish.
    Enqueue(Enqueue),
    /// Withdraw every message still waiting in the queue. Tag `1`.
    ///
    /// Answered once — by a
    /// [`dequeue::Frame`](crate::endpoints::agentic_loop::run::server::channel_response::dequeue::Frame)
    /// saying whether the queue held anything — and then the finish.
    /// Each message it withdraws is ALSO answered, on its own
    /// enqueue channel.
    Dequeue(Dequeue),
    /// The caller's half of a database connection. Tag `2`.
    ///
    /// Quotes the id from the provider's
    /// [`server::channel_request::Frame::Postgres`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::Postgres)
    /// and asks for everything the container writes on it — answered
    /// as [`postgres`](crate::endpoints::agentic_loop::run::server::channel_response::postgres)
    /// frames until the container's socket ends, which is the finish.
    /// See [`Postgres`] for why a connection takes two channels.
    Postgres(Postgres),
}

/// Tag for [`Frame::Enqueue`].
const ENQUEUE: u8 = 0;

/// Tag for [`Frame::Dequeue`].
const DEQUEUE: u8 = 1;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 2;

/// A tag, then that variant's own bytes.
impl Encode for Frame {
    /// The ordinary JSON failure, the enqueue's own. The tag cannot
    /// fail, and neither can the two variants that are not JSON.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Enqueue(request) => {
                out.extend_from_slice(&[ENQUEUE]);
                request.encode(out)
            }
            Frame::Dequeue(request) => {
                out.extend_from_slice(&[DEQUEUE]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Postgres(postgres) => {
                out.extend_from_slice(&[POSTGRES]);
                // Four known bytes; `Infallible` likewise.
                postgres.encode(out).map_err(|error| match error {})
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ENQUEUE => Enqueue::decode(rest)
                .map(Frame::Enqueue)
                .map_err(FrameError::Body),
            DEQUEUE => Ok(Frame::Dequeue(
                Dequeue::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            POSTGRES => Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    ///
    /// What a client newer than its provider produces, which is the
    /// case the tag exists to make survivable: a reader that does not
    /// know a variant says so, rather than reading somebody else's
    /// bytes as its own.
    UnknownTag(u8),
    /// The payload after the tag did not parse.
    Body(serde_json::Error),
    /// The write request was not a connection id.
    Postgres(super::PostgresError),
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
            FrameError::Postgres(error) => {
                write!(f, "postgres write request did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
