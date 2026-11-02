//! A terminal's output, as a stream.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;

use crate::shared::containers::terminal::output::{Frame, FrameError, Owned};
use crate::wire::decode::Decode as _;
use crate::wire::frame;

/// The terminal's frames, in order, as [`terminal`](crate::shared::containers::terminal)
/// states them, an `Error` among them; it ends at the scope's finish.
#[must_use = "a scope that is not polled stalls the shell it reads"]
#[derive(Debug)]
pub struct ExecuteStream {
    receiver: Option<crate::wire::credit::Receiver>,
}

impl ExecuteStream {
    pub(super) fn new(receiver: crate::wire::credit::Receiver) -> Self {
        ExecuteStream { receiver: Some(receiver) }
    }

    fn end(&mut self, error: ExecuteStreamError) -> Poll<Option<Result<Owned, ExecuteStreamError>>> {
        self.receiver = None;
        Poll::Ready(Some(Err(error)))
    }
}

impl Stream for ExecuteStream {
    type Item = Result<Owned, ExecuteStreamError>;

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
        match Frame::decode(payload) {
            Ok(frame) => Poll::Ready(Some(Ok(frame.owned(&bytes)))),
            Err(error) => self.end(ExecuteStreamError::Response(error)),
        }
    }
}

/// A terminal's stream that stopped without its finish.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The connection ended with the scope open: the container stopped.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// A frame that does not belong on a main stream.
    Misrouted,
    /// A frame of the terminal's stream did not decode.
    Response(FrameError),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => f.write_str("the proxy connection ended with the scope open"),
            ExecuteStreamError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteStreamError::Misrouted => f.write_str("a frame that cannot be on a main stream arrived"),
            ExecuteStreamError::Response(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Response(error) => Some(error),
            ExecuteStreamError::Closed | ExecuteStreamError::Misrouted => None,
        }
    }
}
