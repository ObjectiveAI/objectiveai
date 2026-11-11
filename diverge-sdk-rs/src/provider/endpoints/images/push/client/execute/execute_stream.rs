//! The push, as it is told.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;

use super::super::super::server::response;
use crate::shared::error::Error;
use crate::wire::decode::Decode as _;
use crate::wire::frame;

/// The push, one [`response::Frame`] per item, in the order the
/// provider sends them: zero or more [`Blob`](response::Frame::Blob)
/// answers, then the [`Pushed`](response::Frame::Pushed) image or the
/// word that the caller owns it already,
/// [`Exists`](response::Frame::Exists), then the end. A refusal and a
/// provider's error are the last item, as an [`ExecuteStreamError`],
/// with nothing after it.
#[must_use = "a scope that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ExecuteStream {
    /// The scope's responses, until the finish or an ending.
    receiver: Option<crate::wire::credit::Receiver>,
}

impl ExecuteStream {
    /// Over the scope's responses.
    pub(super) fn new(receiver: crate::wire::credit::Receiver) -> Self {
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
            Ok(response::Frame::InsufficientCapacity) => self.end(ExecuteStreamError::InsufficientCapacity),
            Ok(response::Frame::Unavailable) => self.end(ExecuteStreamError::Unavailable),
            Ok(response::Frame::ProofFailed) => self.end(ExecuteStreamError::ProofFailed),
            Ok(response::Frame::Malformed) => self.end(ExecuteStreamError::Malformed),
            Ok(response::Frame::Unsupported) => self.end(ExecuteStreamError::Unsupported),
            Ok(response::Frame::Error(error)) => self.end(ExecuteStreamError::Refused(error)),
            Ok(frame) => Poll::Ready(Some(Ok(frame))),
            Err(error) => self.end(ExecuteStreamError::Response(error)),
        }
    }
}

/// Why the stream stopped short of the image pushed.
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
    /// The Store has no room for the image, and nothing moved. The
    /// protocol's defined refusal, not a failure.
    InsufficientCapacity,
    /// No registry the provider uses serves the image, and the caller
    /// did not supply all of it. The protocol's defined refusal, not a
    /// failure.
    Unavailable,
    /// The caller did not prove it holds the image, and nothing is
    /// owned. The protocol's defined refusal, not a failure.
    ProofFailed,
    /// The image is not one the provider takes. The protocol's defined
    /// refusal, not a failure.
    Malformed,
    /// The provider has no Store, and nothing moved. The protocol's
    /// defined refusal, not a failure.
    Unsupported,
    /// The provider ended the push with an error, in its own words.
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Refused(Error),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => f.write_str("the connection ended with the image push open"),
            ExecuteStreamError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteStreamError::Misrouted => f.write_str("a frame that cannot be on a main stream arrived"),
            ExecuteStreamError::Response(error) => write!(f, "{error}"),
            ExecuteStreamError::InsufficientCapacity => f.write_str("the provider has no room for the image"),
            ExecuteStreamError::Unavailable => f.write_str("the image is available from no source the provider may use"),
            ExecuteStreamError::ProofFailed => f.write_str("the caller did not prove it holds the image"),
            ExecuteStreamError::Malformed => f.write_str("the image is not one the provider takes"),
            ExecuteStreamError::Unsupported => f.write_str("the provider has no Store"),
            ExecuteStreamError::Refused(_) => f.write_str("the provider ended the image push with an error"),
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
            | ExecuteStreamError::InsufficientCapacity
            | ExecuteStreamError::Unavailable
            | ExecuteStreamError::ProofFailed
            | ExecuteStreamError::Malformed
            | ExecuteStreamError::Unsupported
            | ExecuteStreamError::Refused(_) => None,
        }
    }
}
