//! The asks, one at a time.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;

use super::super::request::Frame;
use super::{Ask, ExecuteStreamError};
use crate::server::container_client::WebSocket;
use crate::server::messages::{MessageError, Messages};

/// Every ask the container makes, until the connection ends.
///
/// Yields zero or more [`Ask`]s, and then either ends — the proxy
/// closed cleanly — or yields exactly one [`Err`] and ends: the socket
/// failed or stopped, a message was not binary, or a frame would not
/// decode. Each ask was decoded once on arrival, so
/// [`Ask::frame`] does not fail on one this yielded.
#[must_use = "asks that are not polled are asks nobody answers"]
pub struct ExecuteStream {
    messages: Messages<WebSocket>,
}

impl ExecuteStream {
    pub(super) fn new(socket: WebSocket) -> Self {
        ExecuteStream {
            messages: Messages::new(socket),
        }
    }
}

impl Stream for ExecuteStream {
    type Item = Result<Ask, ExecuteStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let item = match ready!(Pin::new(&mut this.messages).poll_next(cx)) {
            None => None,
            Some(Err(error)) => Some(Err(error.into())),
            Some(Ok(payload)) => match Frame::decode(&payload) {
                Ok(frame) => Some(Ok(Ask {
                    channel: frame.channel,
                    payload,
                })),
                Err(error) => {
                    this.messages.end();
                    Some(Err(ExecuteStreamError::Frame(error)))
                }
            },
        };
        Poll::Ready(item)
    }
}

impl From<MessageError> for ExecuteStreamError {
    fn from(error: MessageError) -> Self {
        match error {
            MessageError::Text => ExecuteStreamError::Text,
            MessageError::Socket(error) => ExecuteStreamError::Socket(error),
            MessageError::Closed => ExecuteStreamError::Closed,
        }
    }
}
