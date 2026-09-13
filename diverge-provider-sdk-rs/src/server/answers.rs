//! A channel this end opened, read as the stream of what answers it.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;

use super::answer::{Answer, answer};
use super::channel::Channel;
use super::scope_handle::ScopeHandle;

/// The payloads a caller sends back on one server-opened channel,
/// in order, until its finish.
///
/// Every item is one response's payload, a refcounted view into the
/// frame it arrived in. The stream ends at the finish; a receiver
/// that closes without one — the connection or the scope went first
/// — yields exactly one [`Err`] and ends, so a reader can tell
/// content that is whole from content that stopped.
pub(crate) struct Answers {
    first: Option<Bytes>,
    channel: Option<Channel>,
}

/// The channel closed without its finish: whatever was being sent
/// did not all arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Unfinished;

impl Answers {
    /// Open the channel `payload` asks for; the stream is its answers.
    pub(crate) async fn open(scope: &ScopeHandle, payload: &[u8]) -> Self {
        Answers {
            first: None,
            channel: Some(scope.send_channel_request(payload).await),
        }
    }

    /// Open the channel and wait for its first answer: `None` for a
    /// finish before any — the wire's could-not-serve — or a caller
    /// that is gone; otherwise the stream, that first answer at its
    /// front.
    pub(crate) async fn first(scope: &ScopeHandle, payload: &[u8]) -> Option<Self> {
        let mut channel = scope.send_channel_request(payload).await;
        loop {
            let bytes = channel.response_receiver.recv().await?;
            match answer(&bytes) {
                Some(Answer::Frame(payload)) => {
                    return Some(Answers {
                        first: Some(payload),
                        channel: Some(channel),
                    });
                }
                Some(Answer::Finish) => return None,
                None => {}
            }
        }
    }
}

impl Stream for Answers {
    type Item = Result<Bytes, Unfinished>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(first) = this.first.take() {
            return Poll::Ready(Some(Ok(first)));
        }
        loop {
            let Some(channel) = this.channel.as_mut() else {
                return Poll::Ready(None);
            };
            let Some(bytes) = ready!(channel.response_receiver.poll_recv(cx)) else {
                this.channel = None;
                return Poll::Ready(Some(Err(Unfinished)));
            };
            match answer(&bytes) {
                Some(Answer::Frame(payload)) => return Poll::Ready(Some(Ok(payload))),
                Some(Answer::Finish) => {
                    this.channel = None;
                    return Poll::Ready(None);
                }
                None => {}
            }
        }
    }
}

impl std::fmt::Debug for Answers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answers").field("ended", &self.channel.is_none()).finish_non_exhaustive()
    }
}
