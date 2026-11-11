//! The scope's end, awaited.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;

use super::super::super::server::response;
use crate::wire::decode::Decode as _;
use crate::wire::frame;

/// The announce scope, read to its end. The rules:
///
/// 1. **Nothing while it stands.** The provider sends no response
///    while the announce stands, so the stream yields nothing for as
///    long as the caller is listed.
/// 2. **The finish is the end.** It ends — `None` — at the provider's
///    finish, which follows the caller's cancel: the caller has left
///    the listing.
/// 3. **One item at most.** A refusal, or a failure of the machinery
///    underneath, is the stream's one item, an [`ExecuteStreamError`],
///    and `None` follows it: the caller is not listed by this scope.
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
    fn end(&mut self, error: ExecuteStreamError) -> Poll<Option<ExecuteStreamError>> {
        self.receiver = None;
        Poll::Ready(Some(error))
    }
}

impl Stream for ExecuteStream {
    type Item = ExecuteStreamError;

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
    Response(response::FrameError),
    /// This connection has an announce open already, and nothing is
    /// announced by this one: one at a time. The protocol's defined
    /// refusal, not a failure.
    AlreadyOpen,
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => f.write_str("the connection ended with the announce open"),
            ExecuteStreamError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteStreamError::Misrouted => f.write_str("a frame that cannot be on a main stream arrived"),
            ExecuteStreamError::Response(error) => write!(f, "{error}"),
            ExecuteStreamError::AlreadyOpen => f.write_str("an announce is open on this connection already"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Response(error) => Some(error),
            ExecuteStreamError::Closed | ExecuteStreamError::Misrouted | ExecuteStreamError::AlreadyOpen => None,
        }
    }
}
