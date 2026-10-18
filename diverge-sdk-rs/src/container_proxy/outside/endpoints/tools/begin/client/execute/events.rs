//! The container's activity, off the main stream.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::response;
use crate::wire::decode::Decode as _;
use crate::provider::endpoints::containers::tools::run::client::execute::Event;
use crate::wire::frame;
use crate::shared::error::Error;

/// Everything the begin scope's main stream says after `Begun`, for
/// as long as the connection lives: the proxy's word whenever an
/// exchange is in flight where none was, and whenever the last in
/// flight has been answered — the latter with the rotating secrets
/// the program handed back — each one [`Event`].
///
/// Zero or more [`Ok`], then either the end — the proxy finished the
/// scope, which is the proxy ending — or exactly one [`Err`] and the
/// end. Every error is terminal. Told apart so that a proxy that
/// finished is not mistaken for a connection that died. There is no
/// timeout: a quiet stream is a container idle, or one serving.
#[must_use = "activity that is not polled grows a queue nobody reads, and the begin's end is never heard"]
#[derive(Debug)]
pub struct Events {
    receiver: Option<UnboundedReceiver<Bytes>>,
}

impl Events {
    pub(super) fn new(receiver: UnboundedReceiver<Bytes>) -> Self {
        Events {
            receiver: Some(receiver),
        }
    }

    fn end(&mut self, error: EventsError) -> Poll<Option<Result<Event, EventsError>>> {
        self.receiver = None;
        Poll::Ready(Some(Err(error)))
    }
}

impl Stream for Events {
    type Item = Result<Event, EventsError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(receiver) = &mut self.receiver else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(receiver.poll_recv(cx)) else {
            return self.end(EventsError::Closed);
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => return self.end(EventsError::Frame(error)),
        };
        let payload = match envelope {
            frame::server::ServerFrame::Response { payload, .. } => payload,
            frame::server::ServerFrame::ResponseFinish { .. } => {
                self.receiver = None;
                return Poll::Ready(None);
            }
            _ => return self.end(EventsError::Misrouted),
        };
        match response::Frame::decode(payload) {
            Ok(response::Frame::Active) => Poll::Ready(Some(Ok(Event::Active))),
            Ok(response::Frame::Inactive(values)) => Poll::Ready(Some(Ok(Event::Inactive(values)))),
            Ok(response::Frame::Error(error)) => self.end(EventsError::Refused(error)),
            // `Begun` comes once, and it came before this stream
            // existed.
            Ok(response::Frame::Begun(_)) => self.end(EventsError::Misrouted),
            Err(error) => self.end(EventsError::Response(error)),
        }
    }
}

/// Why the activity stopped short of the proxy's finish.
#[derive(Debug)]
pub enum EventsError {
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

impl fmt::Display for EventsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventsError::Closed => f.write_str("the proxy connection ended with the begin scope open"),
            EventsError::Frame(error) => write!(f, "frame did not decode: {error}"),
            EventsError::Misrouted => f.write_str("a frame that cannot follow begun arrived on the main stream"),
            EventsError::Response(error) => write!(f, "{error}"),
            EventsError::Refused(_) => f.write_str("the proxy ended the begin scope with an error"),
        }
    }
}

impl std::error::Error for EventsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EventsError::Frame(error) => Some(error),
            EventsError::Response(error) => Some(error),
            EventsError::Closed | EventsError::Misrouted | EventsError::Refused(_) => None,
        }
    }
}
