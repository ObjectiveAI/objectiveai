//! Zero or more answers, then the finish.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{Cancel, one_shot};
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::encode::Encode;
use crate::wire::frame;

/// How a stream's item is read out of one response: handed the whole
/// message, for an item that borrows a piece of it, and the payload
/// after the envelope, for one that parses.
pub type Decoder<T, E> = fn(&Bytes, &[u8]) -> Result<T, E>;

/// Open a streaming scope and hand back its stream, without reading
/// anything: the daemon's first frame is the stream's.
pub async fn execute<Q, T, E>(handle: &Handle, request: &Q, decode: Decoder<T, E>) -> Result<ExecuteStream<T, E>, OpenError<Q::Error>>
where
    Q: Encode,
{
    let scope = one_shot::open::<Q, ()>(handle, request).await.map_err(OpenError::from)?;
    Ok(ExecuteStream::new(scope.response_receiver, decode))
}

/// Open a streaming scope the client may cancel, and hand back the
/// stream and its [`Cancel`].
pub async fn execute_cancellable<Q, T, E>(
    handle: &Handle,
    request: &Q,
    decode: Decoder<T, E>,
) -> Result<(ExecuteStream<T, E>, Cancel), OpenError<Q::Error>>
where
    Q: Encode,
{
    let scope = one_shot::open::<Q, ()>(handle, request).await.map_err(OpenError::from)?;
    let cancel = Cancel::new(handle.clone(), scope.scope);
    Ok((ExecuteStream::new(scope.response_receiver, decode), cancel))
}

/// What the daemon sends on a streaming scope: zero or more responses,
/// each decoded to a `T`, then the end — the daemon's finish. An item
/// that is the daemon's own `Forbidden` or `Error` is a `T` like any
/// other, since the daemon said it; an [`Err`] is this end's, and
/// terminal. There is no timeout: a quiet scope is a scope still
/// running.
#[must_use = "a scope that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ExecuteStream<T, E> {
    receiver: Option<UnboundedReceiver<Bytes>>,
    decode: Decoder<T, E>,
}

impl<T, E> ExecuteStream<T, E> {
    pub(crate) fn new(receiver: UnboundedReceiver<Bytes>, decode: Decoder<T, E>) -> Self {
        ExecuteStream { receiver: Some(receiver), decode }
    }

    fn end(&mut self, error: StreamError<E>) -> Poll<Option<Result<T, StreamError<E>>>> {
        self.receiver = None;
        Poll::Ready(Some(Err(error)))
    }
}

impl<T, E> Stream for ExecuteStream<T, E>
where
    T: Unpin,
    E: Unpin,
{
    type Item = Result<T, StreamError<E>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(receiver) = &mut self.receiver else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(receiver.poll_recv(cx)) else {
            return self.end(StreamError::Closed);
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => return self.end(StreamError::Frame(error)),
        };
        let payload = match envelope {
            frame::server::ServerFrame::Response { payload, .. } => payload,
            frame::server::ServerFrame::ResponseFinish { .. } => {
                self.receiver = None;
                return Poll::Ready(None);
            }
            _ => return self.end(StreamError::Misrouted),
        };
        match (self.decode)(&bytes, payload) {
            Ok(item) => Poll::Ready(Some(Ok(item))),
            Err(error) => self.end(StreamError::Response(error)),
        }
    }
}

/// A streaming scope that did not open.
#[derive(Debug)]
pub enum OpenError<Q> {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The request did not serialize.
    Request(Q),
}

impl<Q, A> From<one_shot::Error<Q, A>> for OpenError<Q> {
    /// Only the two ways an open fails exist before a frame is read,
    /// and [`open`](one_shot::open) produces no other.
    fn from(error: one_shot::Error<Q, A>) -> Self {
        match error {
            one_shot::Error::Send(error) => OpenError::Send(error),
            one_shot::Error::Request(error) => OpenError::Request(error),
            one_shot::Error::Closed
            | one_shot::Error::Frame(_)
            | one_shot::Error::Unanswered
            | one_shot::Error::Misrouted
            | one_shot::Error::Response(_) => unreachable!("an open reads no frame"),
        }
    }
}

impl<Q: fmt::Display> fmt::Display for OpenError<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Send(error) => write!(f, "the request never went out: {error}"),
            OpenError::Request(error) => write!(f, "the request did not serialize: {error}"),
        }
    }
}

impl<Q: std::error::Error + 'static> std::error::Error for OpenError<Q> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::Send(error) => Some(error),
            OpenError::Request(error) => Some(error),
        }
    }
}

/// Why a stream stopped short of the daemon's finish.
#[derive(Debug)]
pub enum StreamError<E> {
    /// The connection went away with the scope still open.
    Closed,
    /// A frame that would not decode.
    Frame(frame::FrameError),
    /// A frame that cannot be on a main stream.
    Misrouted,
    /// A response that is not this scope's.
    Response(E),
}

impl<E: fmt::Display> fmt::Display for StreamError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::Closed => f.write_str("the connection ended with the scope open"),
            StreamError::Frame(error) => write!(f, "frame did not decode: {error}"),
            StreamError::Misrouted => f.write_str("a frame that cannot be on a main stream arrived"),
            StreamError::Response(error) => write!(f, "a response did not parse: {error}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for StreamError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StreamError::Frame(error) => Some(error),
            StreamError::Response(error) => Some(error),
            StreamError::Closed | StreamError::Misrouted => None,
        }
    }
}
