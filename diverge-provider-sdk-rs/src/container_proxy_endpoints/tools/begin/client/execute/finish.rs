//! The begin scope's end, heard.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::response;
use crate::decode::Decode as _;
use crate::frame;
use crate::shared::error::Error;

/// The begin scope's main stream after `Begun`, which carries nothing
/// until the proxy ends it: this resolves when it does.
///
/// [`Ok`] is the proxy's finish — the proxy ending, as it should.
/// [`Err`] is every other ending: the connection gone before the
/// finish, a frame the stream cannot carry, an error the proxy sent
/// after `Begun`. Told apart so that a proxy that finished is not
/// mistaken for a connection that died.
#[must_use = "a begin's end that is not awaited is never heard"]
#[derive(Debug)]
pub struct Finish {
    receiver: UnboundedReceiver<Bytes>,
}

impl Finish {
    pub(super) fn new(receiver: UnboundedReceiver<Bytes>) -> Self {
        Finish { receiver }
    }
}

impl Future for Finish {
    type Output = Result<(), FinishError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(bytes) = ready!(self.receiver.poll_recv(cx)) else {
            return Poll::Ready(Err(FinishError::Closed));
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => return Poll::Ready(Err(FinishError::Frame(error))),
        };
        Poll::Ready(match envelope {
            frame::server::ServerFrame::ResponseFinish { .. } => Ok(()),
            frame::server::ServerFrame::Response { payload, .. } => match response::Frame::decode(payload) {
                Ok(response::Frame::Error(error)) => Err(FinishError::Refused(error)),
                // `Begun` comes once, and it came before this existed.
                Ok(response::Frame::Begun(_)) => Err(FinishError::Misrouted),
                Err(error) => Err(FinishError::Response(error)),
            },
            _ => Err(FinishError::Misrouted),
        })
    }
}

/// Why the begin scope ended some way other than the proxy's finish.
#[derive(Debug)]
pub enum FinishError {
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

impl fmt::Display for FinishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinishError::Closed => f.write_str("the proxy connection ended with the begin scope open"),
            FinishError::Frame(error) => write!(f, "frame did not decode: {error}"),
            FinishError::Misrouted => f.write_str("a frame that cannot follow begun arrived on the main stream"),
            FinishError::Response(error) => write!(f, "{error}"),
            FinishError::Refused(_) => f.write_str("the proxy ended the begin scope with an error"),
        }
    }
}

impl std::error::Error for FinishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FinishError::Frame(error) => Some(error),
            FinishError::Response(error) => Some(error),
            FinishError::Closed | FinishError::Misrouted | FinishError::Refused(_) => None,
        }
    }
}
