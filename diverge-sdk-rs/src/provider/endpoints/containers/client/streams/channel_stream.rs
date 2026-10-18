//! One client-opened channel, read as a stream of what it answers.

use std::fmt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::Answered;
use crate::wire::frame;

/// A channel this end opened, and the answers coming back on it.
///
/// The item is the channel's own: what [`A::decode`](Answered::decode)
/// makes of each frame with the two envelopes taken off. It yields
/// zero or more `Ok`, and then either ends or yields exactly one `Err`
/// and ends. Every error is terminal, which is what makes the type
/// explainable in one line and what [`FusedStream`] then reports
/// honestly.
///
/// [`None`] is the provider finishing the channel as it should — the
/// tree no longer watched, the file all read, the loop over.
/// [`Closed`](ChannelStreamError::Closed) is the connection going away
/// mid-way, and the two are worth telling apart: one says the
/// exchange is over, the other says nothing at all. There is no
/// timeout, here or anywhere in this protocol: a quiet channel is a
/// channel still running.
///
/// # Reading is not optional
///
/// The queue behind this is unbounded, so a stream nobody reads stalls
/// nothing — it grows, at whatever rate the provider is sending, and
/// every frame is kept until somebody takes it or this is dropped.
/// Dropping it stops this end reading and stops nothing else; the
/// provider goes on until the exchange ends on its own terms or the
/// scope does.
#[must_use = "a channel that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ChannelStream<A> {
    /// The channel's responses, until there are no more; [`None`]
    /// once the stream has ended, which is the terminal state. Taken
    /// out rather than flagged so the queue is dropped the moment
    /// nothing will read it.
    receiver: Option<UnboundedReceiver<Bytes>>,
    /// The marker, held as a function type so the stream is `Unpin`,
    /// `Send` and `Sync` whatever the marker is.
    answered: PhantomData<fn() -> A>,
}

impl<A: Answered> ChannelStream<A> {
    /// Take the responses, from the executor that opened the channel.
    pub(crate) fn new(receiver: UnboundedReceiver<Bytes>) -> Self {
        ChannelStream {
            receiver: Some(receiver),
            answered: PhantomData,
        }
    }

    /// End the stream here, and yield this last.
    fn end(&mut self, error: ChannelStreamError<A>) -> Poll<Option<Result<A::Item, ChannelStreamError<A>>>> {
        self.receiver = None;
        Poll::Ready(Some(Err(error)))
    }
}

impl<A: Answered> Stream for ChannelStream<A> {
    type Item = Result<A::Item, ChannelStreamError<A>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let Some(receiver) = this.receiver.as_mut() else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(receiver.poll_recv(cx)) else {
            return this.end(ChannelStreamError::Closed);
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => return this.end(ChannelStreamError::Frame(error)),
        };
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => payload,
            // The finish: the channel ending as it should.
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                this.receiver = None;
                return Poll::Ready(None);
            }
            _ => return this.end(ChannelStreamError::Misrouted),
        };
        // A refcounted view of exactly the payload, so an item can keep
        // the bytes rather than copy them.
        match A::decode(bytes.slice_ref(payload)) {
            Ok(Ok(item)) => Poll::Ready(Some(Ok(item))),
            Ok(Err(refusal)) => this.end(ChannelStreamError::Refused(refusal)),
            Err(error) => this.end(ChannelStreamError::Response(error)),
        }
    }
}

impl<A: Answered> FusedStream for ChannelStream<A> {
    fn is_terminated(&self) -> bool {
        self.receiver.is_none()
    }
}

/// A channel that stopped without ending.
///
/// None of these is the channel finishing: that is [`None`] from the
/// stream. Every one is the last thing the stream yields.
#[derive(Debug)]
pub enum ChannelStreamError<A: Answered> {
    /// The connection ended mid-way: the frames stopped without a
    /// finish, and what was happening is unknown, not over.
    Closed,
    /// What came back was not a frame. Unreachable through this
    /// crate's own router, which decodes the same bytes first; here
    /// because a channel's receiver is public and could be fed by
    /// something else.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on a channel's stream —
    /// neither a channel response nor its finish. Reported rather than
    /// treated as the end, so a channel that BROKE never arrives as one
    /// that ENDED.
    Misrouted,
    /// A frame did not parse.
    Response(A::Error),
    /// The provider said it will not, or cannot, go on — an answer of
    /// a kind, but not one of the channel's items, which is why it is
    /// not the stream simply ending.
    Refused(A::Refusal),
}

impl<A: Answered> fmt::Display for ChannelStreamError<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChannelStreamError::Closed => {
                f.write_str("connection ended in the middle of a channel")
            }
            ChannelStreamError::Frame(error) => write!(f, "channel frame did not decode: {error}"),
            ChannelStreamError::Misrouted => {
                f.write_str("a frame arrived that does not belong on a channel")
            }
            ChannelStreamError::Response(error) => {
                write!(f, "channel frame did not parse: {error}")
            }
            ChannelStreamError::Refused(_) => f.write_str("the provider refused to go on"),
        }
    }
}

impl<A: Answered> std::error::Error for ChannelStreamError<A> {
    /// [`Refused`](ChannelStreamError::Refused) has no source: what it
    /// carries is the wire's own refusal, which a caller matches on
    /// rather than chains.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ChannelStreamError::Frame(error) => Some(error),
            ChannelStreamError::Response(error) => Some(error),
            ChannelStreamError::Closed
            | ChannelStreamError::Misrouted
            | ChannelStreamError::Refused(_) => None,
        }
    }
}
