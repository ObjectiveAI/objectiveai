//! The answers on one channel, decoded as they arrive.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::Receiver;

use super::response_stream::{Kind, ResponseStream};
use super::response_stream_error::ResponseStreamError;

/// A channel's answers, with both envelopes taken off.
///
/// The same thing
/// [`ScopeResponseStream`](super::scope_response_stream::ScopeResponseStream)
/// is, one level down. A channel is one exchange inside a scope — a
/// file being read, a registry blob being served — and what comes back
/// on it is a stream for the same reason: a layer is chunks, and a
/// caller wants them as they land rather than assembled first.
///
/// What an endpoint supplies is one function turning a payload into an
/// item. Everything else is here and written once.
///
/// # Zero or more answers, then one ending
///
/// It yields zero or more [`Ok`], and then either ends or yields
/// exactly one [`Err`] and ends. Every error is terminal, and
/// [`FusedStream`] reports it.
///
/// [`None`] is the channel's answer finishing as it should.
///
/// # A scope ending looks like a closed connection
///
/// [`ResponseStreamError::Closed`] covers both, and on a channel that
/// is not a gap in the reporting so much as a fact about the protocol:
/// a scope ending takes its channels with it, so a
/// [`Router`](super::router::Router) drops this receiver along with the
/// scope's and no
/// [`ChannelResponseFinish`](crate::frame::server::ServerFrame::ChannelResponseFinish)
/// is ever sent. Which is correct — the answer was to a question inside
/// a scope that no longer exists — and it is why a caller reading a
/// channel should be watching its scope too.
///
/// # Reading is not optional
///
/// The queue is bounded, at whatever depth the channel was opened with.
/// A channel nobody reads stops the router that many frames later, and
/// stopping the router stops every scope on the connection rather than
/// only this one.
///
/// It matters more here than on a scope. What rides a channel is
/// usually the bulk — an image layer, a database connection, a
/// command's items — so a channel falls behind at a rate a scope's own
/// answers rarely reach.
///
/// # Dropping this stops you reading, not the provider writing
///
/// A channel ends when whoever is ANSWERING finishes it, so a caller
/// that walks away has said nothing to the far end. The provider keeps
/// sending, the router keeps discarding, and the channel number is not
/// reclaimed until a finish that may never come.
#[must_use = "a response stream that is not polled stalls every scope on the connection"]
pub struct ChannelResponseStream<T, E>(ResponseStream<T, E>);

impl<T, E> ChannelResponseStream<T, E> {
    /// Take a channel's receiver and the thing that reads what comes
    /// out of it.
    ///
    /// `decode` gets one payload with the envelope already off, as
    /// owned [`Bytes`] so that an item can keep it without copying,
    /// and returns the item or the endpoint's own error.
    ///
    /// Folding a provider's error frame into that error is the
    /// endpoint's to do, because telling one from an answer needs to
    /// know what an answer looks like.
    pub fn new(
        responses: Receiver<Bytes>,
        decode: fn(Bytes) -> Result<T, E>,
    ) -> Self {
        ChannelResponseStream(ResponseStream::new(
            responses,
            decode,
            Kind::Channel,
        ))
    }
}

/// One answer at a time, until there are no more.
///
/// See the type's own documentation for what ends it and what that
/// means.
impl<T, E> Stream for ChannelResponseStream<T, E> {
    type Item = Result<T, ResponseStreamError<E>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // A channel end, a function pointer and a byte — all `Unpin`,
        // so this never has to project.
        self.get_mut().0.poll(cx)
    }
}

/// Whether the stream is over.
impl<T, E> FusedStream for ChannelResponseStream<T, E> {
    fn is_terminated(&self) -> bool {
        self.0.is_terminated()
    }
}

/// What can be shown of it, which is not the decoder.
impl<T, E> fmt::Debug for ChannelResponseStream<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ChannelResponseStream").field(&self.0).finish()
    }
}
