//! What a laboratory's MCP server says on its own account.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use rmcp::model::ServerNotification;
use tokio::sync::mpsc::UnboundedReceiver;

use super::McpError;
use super::super::super::server::channel_response;
use crate::decode::Decode;
use crate::frame;

/// Notifications, for as long as the server has any.
///
/// What [`notifications`](super::ExecuteHandle::notifications) gives
/// back. The ask has gone out and the router is already putting what
/// arrives where this will find it.
///
/// The item is a [`ServerNotification`] — tools changed, resources
/// changed, a resource updated, a log line. See
/// [`ServerNotification`] for the whole of what one can be.
///
/// # It is the one ask that is not answered
///
/// The other four are asked once and answered once. This is not an
/// answer: it is the place a server pushes into when something changes,
/// and it produces for as long as it is held.
///
/// Which is also why there is nothing to unsubscribe with. Dropping
/// this drops the channel, and that is what tells the provider nobody
/// is listening.
///
/// # Zero or more notifications, then one ending
///
/// A server with nothing to report produces nothing and then finishes,
/// which is not distinguishable from one that had plenty to say and has
/// stopped — nor should it be. Both are a stream that ended.
///
/// [`None`] is that ending. Everything else is an
/// [`McpError`], and every one of those is terminal: a stream that has
/// said it could not read what it was given does not go on reading.
pub struct McpNotificationStream {
    /// The channel's answers, until there are no more of them.
    ///
    /// [`None`] once the stream has ended, which is the terminal state
    /// and the whole of it.
    ///
    /// # It does two jobs, and the second is the load-bearing one
    ///
    /// [`poll_recv`](UnboundedReceiver::poll_recv) latches [`None`]
    /// forever once the senders are gone, so a stream that reported
    /// that as an error each time it saw it would report it without
    /// end. That is the obvious job.
    ///
    /// The other: after a decode failure the channel is still open and
    /// may still be full of frames. Nothing about the receiver stops a
    /// stream that has declared itself finished from going on to yield
    /// notifications it has already said it could not read, and this
    /// does.
    response_receiver: Option<UnboundedReceiver<Bytes>>,
}

impl McpNotificationStream {
    /// Take the channel's answers, from the
    /// [`notifications`](super::ExecuteHandle::notifications) that
    /// asked for them.
    ///
    /// Not public. A notification stream exists because a request went
    /// out, so the only thing that can honestly make one of these is
    /// the thing that sent it.
    pub(super) fn new(response_receiver: UnboundedReceiver<Bytes>) -> Self {
        McpNotificationStream {
            response_receiver: Some(response_receiver),
        }
    }
}

/// One notification at a time, until there are no more.
///
/// See the type's own documentation for what ends it and what that
/// means.
impl Stream for McpNotificationStream {
    type Item = Result<ServerNotification, McpError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // One channel end, and it is `Unpin`, so this never has to
        // project.
        let this = self.get_mut();
        let Some(responses) = this.response_receiver.as_mut() else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(responses.poll_recv(cx)) else {
            this.response_receiver = None;
            return Poll::Ready(Some(Err(McpError::Unanswered)));
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(McpError::Frame(error))));
            }
        };
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => {
                payload
            }
            // The finish, which is the server saying it will push no
            // more. The ordinary end.
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                this.response_receiver = None;
                return Poll::Ready(None);
            }
            _ => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(McpError::Misrouted)));
            }
        };
        Poll::Ready(Some(
            match channel_response::mcp_notifications::Frame::decode(payload) {
                Ok(channel_response::mcp_notifications::Frame::Notification(
                    notification,
                )) => Ok(notification),
                Ok(channel_response::mcp_notifications::Frame::Error(
                    error,
                )) => {
                    // The server saying it will push no more, and why.
                    // Nothing follows it.
                    this.response_receiver = None;
                    Err(McpError::Mcp(error))
                }
                Err(error) => {
                    this.response_receiver = None;
                    Err(McpError::Answer(error))
                }
            },
        ))
    }
}

impl FusedStream for McpNotificationStream {
    fn is_terminated(&self) -> bool {
        self.response_receiver.is_none()
    }
}
