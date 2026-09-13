//! The agent's conversation, off the main stream.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::response;
use crate::decode::Decode as _;
use crate::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use crate::frame;
use crate::shared::error::Error;

/// Every chunk the agent produces, for as long as the connection
/// lives: the begin scope's main stream after `Begun`.
///
/// Zero or more [`Ok`], then either the end — the proxy finished the
/// scope, which is the proxy ending — or exactly one [`Err`] and the
/// end. Every error is terminal. There is no timeout: a quiet
/// conversation is an agent with nothing to say.
#[must_use = "a conversation that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct Chunks {
    receiver: Option<UnboundedReceiver<Bytes>>,
}

impl Chunks {
    pub(super) fn new(receiver: UnboundedReceiver<Bytes>) -> Self {
        Chunks {
            receiver: Some(receiver),
        }
    }

    fn end(&mut self, error: ChunksError) -> Poll<Option<Result<AgenticLoopChunk, ChunksError>>> {
        self.receiver = None;
        Poll::Ready(Some(Err(error)))
    }
}

impl Stream for Chunks {
    type Item = Result<AgenticLoopChunk, ChunksError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(receiver) = &mut self.receiver else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(receiver.poll_recv(cx)) else {
            return self.end(ChunksError::Closed);
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => return self.end(ChunksError::Frame(error)),
        };
        let payload = match envelope {
            frame::server::ServerFrame::Response { payload, .. } => payload,
            frame::server::ServerFrame::ResponseFinish { .. } => {
                self.receiver = None;
                return Poll::Ready(None);
            }
            _ => return self.end(ChunksError::Misrouted),
        };
        match response::Frame::decode(payload) {
            Ok(response::Frame::Chunk(chunk)) => Poll::Ready(Some(Ok(chunk))),
            Ok(response::Frame::Error(error)) => self.end(ChunksError::Refused(error)),
            // `Begun` comes once, and it came before this stream
            // existed.
            Ok(response::Frame::Begun) => self.end(ChunksError::Misrouted),
            Err(error) => self.end(ChunksError::Response(error)),
        }
    }
}

/// Why the conversation stopped short of the proxy's finish.
#[derive(Debug)]
pub enum ChunksError {
    /// The connection went away with the scope still open.
    Closed,
    /// A frame that would not decode.
    Frame(frame::FrameError),
    /// A frame that cannot be on a main stream after `Begun`.
    Misrouted,
    /// A response that is not this scope's.
    Response(response::FrameError),
    /// The proxy ended the scope with an error.
    Refused(Error),
}

impl fmt::Display for ChunksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunksError::Closed => f.write_str("the proxy connection ended with the begin scope open"),
            ChunksError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ChunksError::Misrouted => f.write_str("a frame that cannot follow begun arrived on the main stream"),
            ChunksError::Response(error) => write!(f, "{error}"),
            ChunksError::Refused(_) => f.write_str("the proxy ended the begin scope with an error"),
        }
    }
}

impl std::error::Error for ChunksError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ChunksError::Frame(error) => Some(error),
            ChunksError::Response(error) => Some(error),
            ChunksError::Closed | ChunksError::Misrouted | ChunksError::Refused(_) => None,
        }
    }
}
