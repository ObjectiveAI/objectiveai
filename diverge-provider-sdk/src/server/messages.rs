//! A proxy socket as the binary messages it carries, with its two
//! endings told apart.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use tokio_tungstenite::tungstenite::{self, Message};

/// The binary messages on a proxy socket, until it ends.
///
/// Every executor that reads a path reads through one of these. Pings,
/// pongs and raw frames are skipped; a text message is the far side
/// speaking something else, and ends the stream with
/// [`Text`](MessageError::Text). A Close frame ends the stream with
/// [`None`] — the clean close, the wire's "complete" — while the
/// socket failing or simply stopping ends it with
/// [`Socket`](MessageError::Socket) or [`Closed`](MessageError::Closed),
/// the abrupt end, the wire's "died". Every error is terminal, and the
/// stream stays ended.
pub(crate) struct Messages<S> {
    /// The socket, or nothing once the stream has ended.
    inner: Option<S>,
}

impl<S> Messages<S> {
    pub(crate) fn new(inner: S) -> Self {
        Messages { inner: Some(inner) }
    }

    /// End the stream from this side: whatever ended it is the
    /// caller's to report.
    pub(crate) fn end(&mut self) {
        self.inner = None;
    }
}

impl<S> Stream for Messages<S>
where
    S: Stream<Item = Result<Message, tungstenite::Error>> + Unpin,
{
    type Item = Result<Bytes, MessageError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            let Some(inner) = this.inner.as_mut() else {
                return Poll::Ready(None);
            };
            let item = match ready!(Pin::new(inner).poll_next(cx)) {
                Some(Ok(Message::Binary(bytes))) => Some(Ok(bytes)),
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {
                    continue;
                }
                Some(Ok(Message::Close(_))) => None,
                Some(Ok(Message::Text(_))) => Some(Err(MessageError::Text)),
                Some(Err(error)) => Some(Err(MessageError::Socket(error))),
                None => Some(Err(MessageError::Closed)),
            };
            if !matches!(item, Some(Ok(_))) {
                this.inner = None;
            }
            return Poll::Ready(item);
        }
    }
}

/// Why a proxy socket ended other than cleanly.
#[derive(Debug)]
pub(crate) enum MessageError {
    /// A text message: the far side speaking something else.
    Text,
    /// The socket failed.
    Socket(tungstenite::Error),
    /// The socket ended without a Close: the far side died.
    Closed,
}
