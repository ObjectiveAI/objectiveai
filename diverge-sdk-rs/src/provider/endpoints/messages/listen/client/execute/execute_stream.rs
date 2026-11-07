//! The messages, as they are sent.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;

use super::super::super::server::response;
use crate::wire::decode::Decode as _;
use crate::wire::frame;

/// The messages sent to the caller, one [`response::Message`] per
/// item, in the order the provider sends them. It ends — `None` — at
/// the provider's finish. A refusal, or a failure of the machinery
/// underneath, is the stream's last item, and `None` follows it.
#[must_use = "a scope that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ExecuteStream {
    /// The scope's responses, until the finish or an ending.
    receiver: Option<crate::wire::credit::Receiver>,
}

impl ExecuteStream {
    /// Over the scope's responses.
    pub(super) fn new(receiver: crate::wire::credit::Receiver) -> Self {
        ExecuteStream { receiver: Some(receiver) }
    }

    /// End the stream with `error` as its last item.
    fn end(&mut self, error: ExecuteStreamError) -> Poll<Option<Result<response::Message, ExecuteStreamError>>> {
        self.receiver = None;
        Poll::Ready(Some(Err(error)))
    }
}

impl Stream for ExecuteStream {
    type Item = Result<response::Message, ExecuteStreamError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(receiver) = &mut self.receiver else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(receiver.poll_recv(cx)) else {
            return self.end(ExecuteStreamError::Closed);
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => return self.end(ExecuteStreamError::Frame(error)),
        };
        let payload = match envelope {
            frame::server::ServerFrame::Response { payload, .. } => payload,
            frame::server::ServerFrame::ResponseFinish { .. } => {
                self.receiver = None;
                return Poll::Ready(None);
            }
            _ => return self.end(ExecuteStreamError::Misrouted),
        };
        match response::Frame::decode(payload) {
            Ok(response::Frame::Message(message)) => Poll::Ready(Some(Ok(message))),
            Ok(response::Frame::NotDiscoverable) => self.end(ExecuteStreamError::NotDiscoverable),
            Ok(response::Frame::Unsupported) => self.end(ExecuteStreamError::Unsupported),
            Ok(response::Frame::AlreadyOpen) => self.end(ExecuteStreamError::AlreadyOpen),
            Err(error) => self.end(ExecuteStreamError::Response(error)),
        }
    }
}

/// Why the stream stopped short of the provider's finish.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The connection went away with the scope still open.
    Closed,
    /// A frame that would not decode.
    Frame(frame::FrameError),
    /// A frame that cannot be on a main stream.
    Misrouted,
    /// A response that is not this scope's.
    Response(response::FrameDecodeError),
    /// The caller's connection is not in discovery, and nothing is
    /// listened for: only a discoverable identity listens. The
    /// protocol's defined refusal, not a failure.
    NotDiscoverable,
    /// The provider relays no messages, and nothing is listened for.
    /// The protocol's defined refusal, not a failure.
    Unsupported,
    /// This connection, or another under the caller's identity, has a
    /// listen open already, and nothing is listened for: one at a time.
    /// The protocol's defined refusal, not a failure.
    AlreadyOpen,
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => f.write_str("the connection ended with the listen open"),
            ExecuteStreamError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteStreamError::Misrouted => f.write_str("a frame that cannot be on a main stream arrived"),
            ExecuteStreamError::Response(error) => write!(f, "{error}"),
            ExecuteStreamError::NotDiscoverable => f.write_str("this connection is not in the provider's discovery"),
            ExecuteStreamError::Unsupported => f.write_str("the provider relays no messages"),
            ExecuteStreamError::AlreadyOpen => f.write_str("a listen is open under this identity already"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Response(error) => Some(error),
            ExecuteStreamError::Closed
            | ExecuteStreamError::Misrouted
            | ExecuteStreamError::NotDiscoverable
            | ExecuteStreamError::Unsupported
            | ExecuteStreamError::AlreadyOpen => None,
        }
    }
}
