//! What the container wrote, as it wrote it.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::SplitStream;

use super::ExecuteStreamError;
use crate::server::container_client::ContainerWebSocket;
use crate::server::messages::{MessageError, Messages};

/// pgwire from the container, one message one chunk, until its socket
/// ends.
///
/// Yields zero or more chunks, and then either ends — the driver hung
/// up, and the proxy closed cleanly — or yields exactly one [`Err`]
/// and ends: the socket failed or stopped. Nothing here is parsed; a
/// pgwire message larger than one chunk spans several, and the
/// database's side reassembles as from a socket.
#[must_use = "a connection that is not read is a driver nobody answers"]
pub struct ExecuteStream {
    messages: Messages<SplitStream<ContainerWebSocket>>,
}

impl ExecuteStream {
    pub(super) fn new(stream: SplitStream<ContainerWebSocket>) -> Self {
        ExecuteStream {
            messages: Messages::new(stream),
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
            None => None,
            Some(Ok(bytes)) => Some(Ok(bytes)),
            Some(Err(MessageError::Socket(error))) => {
                Some(Err(ExecuteStreamError::Socket(error)))
            }
            Some(Err(MessageError::Closed)) => Some(Err(ExecuteStreamError::Closed)),
        };
        Poll::Ready(item)
    }
}
