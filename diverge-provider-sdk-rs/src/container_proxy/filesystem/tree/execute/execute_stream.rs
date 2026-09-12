//! The tree, event by event.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;

use super::super::response;
use super::ExecuteStreamError;
use crate::server::container_client::ContainerWebSocket;
use crate::server::messages::{MessageError, Messages};
use crate::shared::filetree;

/// The container's filesystem: a snapshot, then every change.
///
/// Yields [`filetree::response::Frame`]s — a
/// [`Snapshot`](filetree::response::Frame::Snapshot) first, again
/// whenever the watch lost events, deltas otherwise — and then either
/// ends, the proxy having closed cleanly, or yields exactly one
/// [`Err`] and ends: the watch could not exist
/// ([`Refused`](ExecuteStreamError::Refused), with the proxy's
/// reason), a frame would not decode, or the socket failed or
/// stopped. Fold them with
/// [`Root::update`](filetree::response::Root::update).
#[must_use = "a watch that is not polled is a tree nobody sees"]
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
    type Item = Result<filetree::response::Frame, ExecuteStreamError>;

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
                Ok(response::Frame::Filetree(frame)) => Some(Ok(frame)),
                Ok(response::Frame::Error(reason)) => {
                    this.messages.end();
                    Some(Err(ExecuteStreamError::Refused(reason.to_owned())))
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
