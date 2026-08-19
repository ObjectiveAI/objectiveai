//! The answers to a request, decoded as they arrive.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::Receiver;

use super::response_stream::{Kind, ResponseStream};
use super::response_stream_error::ResponseStreamError;

/// A scope's answers, with both envelopes taken off.
///
/// For the endpoints that do not collapse into one call. A volume
/// [`watch`](crate::endpoints::volumes::watch) sends a snapshot and
/// then changes to it for as long as the scope stays open; an agentic
/// loop sends chunks until the work is done. Something that returned a
/// single value would have had to pick a frame and throw the rest away.
///
/// What an endpoint supplies is one function turning a payload into an
/// item. Everything else — polling the receiver, reading the envelope,
/// knowing which frame ends the scope, staying ended once it has — is
/// here and written once.
///
/// # Zero or more answers, then one ending
///
/// It yields zero or more [`Ok`], and then either ends or yields
/// exactly one [`Err`] and ends. Every error is terminal, which is what
/// makes the type explainable in one line and what [`FusedStream`] then
/// reports honestly.
///
/// [`None`] is the provider finishing as it should.
/// [`ResponseStreamError::Closed`] is the connection going away
/// mid-stream, and the two are worth telling apart — one says the
/// provider is done, the other says nothing at all.
///
/// There is no timeout, here or anywhere else in this protocol. A
/// provider with nothing to say is a stream that says nothing for as
/// long as that lasts, and a quiet stream is not a finished one.
///
/// # It takes a receiver, not a [`Scope`](super::scope::Scope)
///
/// Because what to do with the REST of a scope is the endpoint's
/// business. A watch drops the scope's request receiver, since nothing
/// should open a channel inside one — and that is not a spare field but
/// a hazard: it is bounded, the router awaits it, so a provider that
/// opened two channels into a receiver nobody reads would block the
/// router forever and stall every scope on the connection.
///
/// An endpoint that DOES expect channels — a laboratory run being
/// served an image — keeps that receiver and reads it. This one would
/// be wrong to decide for either.
///
/// # Reading is not optional
///
/// The queue is bounded, at whatever depth the request was opened with.
/// A stream nobody reads stops the router that many frames later, and
/// stopping the router stops every scope on the connection rather than
/// only this one.
///
/// That obligation gets sharper as a [`Stream`], not softer. A
/// `select!` is exactly where a stream someone means to ignore ends up,
/// and one parked there that rarely wins stalls the connection once its
/// capacity fills.
///
/// # Dropping this stops you reading, not the provider writing
///
/// There is no frame for cancelling a scope. A client opens one and a
/// server ends one; nothing in
/// [`ClientFrame`](crate::frame::client::ClientFrame) says stop. So
/// dropping this leaves the provider sending, the router decoding and
/// discarding a frame at a time, and the scope number unreclaimed —
/// because that only happens on a finish which is never coming.
///
/// For an endpoint that ends by itself this is the rare case of a
/// caller walking away. For one that does not — a watch — it is the
/// ORDINARY exit. It is a gap in the protocol rather than in this type,
/// and it is the strongest argument this crate has for a cancel frame.
#[must_use = "a response stream that is not polled stalls every scope on the connection"]
pub struct ScopeResponseStream<T, E>(ResponseStream<T, E>);

impl<T, E> ScopeResponseStream<T, E> {
    /// Take a scope's response receiver and the thing that reads what
    /// comes out of it.
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
        ScopeResponseStream(ResponseStream::new(responses, decode, Kind::Scope))
    }
}

/// One answer at a time, until there are no more.
///
/// See the type's own documentation for what ends it and what that
/// means.
impl<T, E> Stream for ScopeResponseStream<T, E> {
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
///
/// Worth answering, because a stream that runs until a provider stops
/// is exactly the one that ends up in a `select!`, which wants a fused
/// one.
impl<T, E> FusedStream for ScopeResponseStream<T, E> {
    fn is_terminated(&self) -> bool {
        self.0.is_terminated()
    }
}

/// What can be shown of it, which is not the decoder.
impl<T, E> fmt::Debug for ScopeResponseStream<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ScopeResponseStream").field(&self.0).finish()
    }
}
