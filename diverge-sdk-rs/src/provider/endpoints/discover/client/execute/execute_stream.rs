//! The listing, as it is sent.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;

use super::super::super::server::response;
use crate::wire::decode::Decode as _;
use crate::wire::frame;

/// The provider's responses on a discover scope, decoded, in the
/// order the provider sends them: every discoverable connected
/// identity as added, the word that the listing is whole, and from
/// then on identities added and removed as they connect and
/// disconnect.
///
/// It ends — `None` — at the provider's finish. A failure of the
/// machinery underneath is the stream's last item, and `None` follows
/// it. There is no refusal: a discover listing has no error.
#[must_use = "a scope that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ExecuteStream {
    receiver: Option<crate::wire::credit::Receiver>,
}

impl ExecuteStream {
    pub(super) fn new(receiver: crate::wire::credit::Receiver) -> Self {
        ExecuteStream { receiver: Some(receiver) }
    }

    fn end(&mut self, error: ExecuteStreamError) -> Poll<Option<Result<response::Frame, ExecuteStreamError>>> {
        self.receiver = None;
        Poll::Ready(Some(Err(error)))
    }
}

impl Stream for ExecuteStream {
    type Item = Result<response::Frame, ExecuteStreamError>;

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
            Ok(frame) => Poll::Ready(Some(Ok(frame))),
            Err(error) => self.end(ExecuteStreamError::Response(error)),
        }
    }
}

/// A discover listing that broke. Each is the machinery underneath,
/// not the provider refusing.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The connection ended with the listing open.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// A frame that cannot be on a main stream arrived.
    Misrouted,
    /// A response did not decode as a discover response.
    Response(response::FrameDecodeError),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => f.write_str("the connection ended with the discover listing open"),
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
