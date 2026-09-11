//! The loop, chunk by chunk.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;

use super::super::response;
use super::ExecuteStreamError;
use crate::decode::Decode as _;
use crate::server::container_client::ContainerWebSocket;
use crate::server::messages::{MessageError, Messages};

/// The loop as it happens.
///
/// Yields zero or more [`AgenticLoopChunk`](response::AgenticLoopChunk)s
/// and then either ends — the proxy closed cleanly, the loop over —
/// or yields exactly one [`Err`] and ends: the container's own
/// `Error` frame ([`Refused`](ExecuteStreamError::Refused), first
/// when nothing ran, last when the loop died), a frame that would
/// not decode, or the socket failing or stopping. A fatal
/// notification is not an error here: it is a chunk, the loop's own
/// last word.
#[must_use = "a loop that is not polled is a loop nobody hears"]
pub struct ExecuteStream {
    messages: Messages<ContainerWebSocket>,
}

impl ExecuteStream {
    pub(super) fn new(socket: ContainerWebSocket) -> Self {
        ExecuteStream {
            messages: Messages::new(socket),
        }
    }
}

impl Stream for ExecuteStream {
    type Item = Result<response::AgenticLoopChunk, ExecuteStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let item = match ready!(Pin::new(&mut this.messages).poll_next(cx)) {
            None => None,
            Some(Err(MessageError::Socket(error))) => {
                Some(Err(ExecuteStreamError::Socket(error)))
            }
            Some(Err(MessageError::Closed)) => Some(Err(ExecuteStreamError::Closed)),
            Some(Ok(bytes)) => match response::Frame::decode(&bytes) {
                Ok(response::Frame::Chunk(chunk)) => Some(Ok(chunk)),
                Ok(response::Frame::Error(error)) => {
                    this.messages.end();
                    Some(Err(ExecuteStreamError::Refused(error)))
                }
                Err(error) => {
                    this.messages.end();
                    Some(Err(ExecuteStreamError::Frame(error)))
                }
            },
        };
        Poll::Ready(item)
    }
}
