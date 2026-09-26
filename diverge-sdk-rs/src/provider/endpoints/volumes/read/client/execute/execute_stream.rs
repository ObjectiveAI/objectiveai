//! The file's bytes, as a stream.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::response;
use crate::wire::decode::Decode as _;
use crate::wire::frame;
use crate::shared::error::Error;

/// What the provider sends on the scope: zero or more pieces, then
/// the end — the provider's finish — or exactly one error and the
/// end. Every error is terminal. There is no timeout: a quiet scope
/// is a scope still running.
#[must_use = "a scope that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ExecuteStream {
    receiver: Option<UnboundedReceiver<Bytes>>,
}

impl ExecuteStream {
    pub(super) fn new(receiver: UnboundedReceiver<Bytes>) -> Self {
        ExecuteStream {
            receiver: Some(receiver),
        }
    }

    fn end(
        &mut self,
        error: ExecuteStreamError,
    ) -> Poll<Option<Result<Bytes, ExecuteStreamError>>> {
        self.receiver = None;
        Poll::Ready(Some(Err(error)))
    }
}

impl Stream for ExecuteStream {
    type Item = Result<Bytes, ExecuteStreamError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
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
            Ok(response::Frame::Body(body)) => {
                Poll::Ready(Some(Ok(bytes.slice_ref(body.0))))
            }
            Ok(response::Frame::Error(error)) => {
                self.end(ExecuteStreamError::Refused(error))
            }
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
    /// The file was not read, or not all of it: the provider's own
    /// words. A volume the caller cannot see, one mounted somewhere,
    /// a path that names no file.
    Refused(Error),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => {
                f.write_str("the connection ended with the volume read open")
            }
            ExecuteStreamError::Frame(error) => {
                write!(f, "frame did not decode: {error}")
            }
            ExecuteStreamError::Misrouted => {
                f.write_str("a frame that cannot be on a main stream arrived")
            }
            ExecuteStreamError::Response(error) => write!(f, "{error}"),
            ExecuteStreamError::Refused(_) => {
                f.write_str("the provider ended the volume read with an error")
            }
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
            | ExecuteStreamError::Refused(_) => None,
        }
    }
}
