//! The container's server, notification by notification.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::Stream;
use rmcp::model::ServerNotification;

use super::super::response;
use super::ExecuteStreamError;
use crate::decode::Decode as _;
use crate::server::container_client::ContainerWebSocket;
use crate::server::messages::{MessageError, Messages};

/// The subscription as it happens.
///
/// Yields zero or more [`ServerNotification`]s and then either ends —
/// the proxy closed cleanly, nothing more on this connection — or
/// yields exactly one [`Err`] and ends: the `Error` frame the proxy
/// sends when the server cannot be reached
/// ([`Refused`](ExecuteStreamError::Refused)), a frame that would not
/// decode, or the socket failing or stopping.
#[must_use = "a subscription that is not polled hears nothing"]
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
    type Item = Result<ServerNotification, ExecuteStreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let item = match ready!(Pin::new(&mut this.messages).poll_next(cx)) {
            None => None,
            Some(Err(MessageError::Socket(error))) => Some(Err(ExecuteStreamError::Socket(error))),
            Some(Err(MessageError::Closed)) => Some(Err(ExecuteStreamError::Closed)),
            Some(Ok(bytes)) => match response::Frame::decode(&bytes) {
                Ok(response::Frame::Notification(notification)) => Some(Ok(notification)),
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
