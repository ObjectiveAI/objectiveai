//! The file, piece by piece.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;

use super::super::response;
use super::ExecuteStreamError;
use crate::server::container_client::ContainerWebSocket;
use crate::server::messages::{MessageError, Messages};

/// One file, arriving.
///
/// Yields zero or more pieces — [`Bytes`], sliced out of the message
/// they arrived in rather than copied — and then either ends, the file
/// complete, or yields exactly one [`Err`] and ends. An empty file is
/// one empty piece, then the end. [`Refused`](ExecuteStreamError::Refused)
/// is the proxy saying the file was not read, or not all of it, with
/// its reason; [`Unserved`](ExecuteStreamError::Unserved) is a close
/// with nothing before it, the wire's refusal with nothing to say.
#[must_use = "a read that is not polled is a file nobody receives"]
pub struct ExecuteStream {
    messages: Messages<ContainerWebSocket>,
    /// Whether any body has arrived: a close before one is `Unserved`.
    received: bool,
}

impl ExecuteStream {
    pub(super) fn new(socket: ContainerWebSocket) -> Self {
        ExecuteStream {
            messages: Messages::new(socket),
            received: false,
        }
    }
}

impl Stream for ExecuteStream {
    type Item = Result<Bytes, ExecuteStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let item = match ready!(Pin::new(&mut this.messages).poll_next(cx)) {
            None if this.received => None,
            None => {
                // Latched: the messages already ended, so this is
                // reported once and the stream stays ended.
                this.received = true;
                Some(Err(ExecuteStreamError::Unserved))
            }
            Some(Err(MessageError::Socket(error))) => {
                Some(Err(ExecuteStreamError::Socket(error)))
            }
            Some(Err(MessageError::Closed)) => Some(Err(ExecuteStreamError::Closed)),
            Some(Ok(bytes)) => match response::Frame::decode(&bytes) {
                Ok(response::Frame::Body(_)) => {
                    this.received = true;
                    // The body is everything after the kind byte.
                    Some(Ok(bytes.slice(1..)))
                }
                Ok(response::Frame::Error(reason)) => {
                    this.received = true;
                    this.messages.end();
                    Some(Err(ExecuteStreamError::Refused(reason.to_owned())))
                }
                Err(error) => {
                    this.received = true;
                    this.messages.end();
                    Some(Err(ExecuteStreamError::Frame(error)))
                }
            },
        };
        Poll::Ready(item)
    }
}
