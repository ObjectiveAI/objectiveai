//! The listing, as it is sent.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::response;
use crate::shared::error::Error;
use crate::wire::decode::Decode as _;
use crate::wire::frame;

/// The listing, one [`response::Frame`] per item, in the order the
/// provider sends them: every volume the caller has as added, the
/// word that the listing is whole, and from then on volumes added,
/// changed and removed as the caller creates, edits and deletes
/// them. Ends at the provider's finish — after the stop, or the
/// connection ending — and an error from the provider is the last
/// item, with nothing after it.
#[must_use = "a scope that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ExecuteStream {
    /// The scope's responses, until the finish or an ending.
    receiver: Option<UnboundedReceiver<Bytes>>,
}

impl ExecuteStream {
    /// Over the scope's responses.
    pub(super) fn new(receiver: UnboundedReceiver<Bytes>) -> Self {
        ExecuteStream {
            receiver: Some(receiver),
        }
    }

    /// End the stream with `error` as its last item.
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
            Ok(response::Frame::Error(error)) => self.end(ExecuteStreamError::Refused(error)),
            Ok(frame) => Poll::Ready(Some(Ok(frame))),
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
    /// The provider ended the listing with an error, in its own words.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little; a caller that wants what is inside it
    /// matches the variant.
    Refused(Error),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => f.write_str("the connection ended with the volume listing open"),
            ExecuteStreamError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteStreamError::Misrouted => f.write_str("a frame that cannot be on a main stream arrived"),
            ExecuteStreamError::Response(error) => write!(f, "{error}"),
            ExecuteStreamError::Refused(_) => f.write_str("the provider ended the volume listing with an error"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Response(error) => Some(error),
            ExecuteStreamError::Closed | ExecuteStreamError::Misrouted | ExecuteStreamError::Refused(_) => None,
        }
    }
}
