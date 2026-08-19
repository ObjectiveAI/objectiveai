//! The answers to a request, decoded as they arrive.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::Receiver;

use super::handle::Handle;
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
/// # Dropping this stops you reading, and the provider only if it was
/// told to
///
/// There is still no frame for cancelling a SCOPE. A client opens one
/// and a server ends one; nothing in
/// [`ClientFrame`](crate::frame::client::ClientFrame) says stop at that
/// level. So by default dropping this leaves the provider sending, the
/// router decoding and discarding a frame at a time, and the scope
/// number unreclaimed — because that only happens on a finish which is
/// never coming.
///
/// [`stop_with`](Self::stop_with) is how an endpoint escapes that, and
/// it is endpoint-specific by necessity: what it sends is a CHANNEL
/// request, so it exists only where an endpoint defined one that means
/// stop.
///
/// Which most have no use for. An endpoint that ends by itself is
/// already finishing; dropping its stream early is the rare case of a
/// caller walking away. It is the ones that do not end — a
/// [`watch`](crate::endpoints::volumes::watch) — where dropping is the
/// ORDINARY exit, and those are exactly the ones that define a stop.
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

    /// Say this if the stream is dropped before it ends.
    ///
    /// A channel request — `payload` is one, encoded, tag and all —
    /// sent on `scope` when a caller walks away from a stream that was
    /// still running. For an endpoint that has something meaning stop,
    /// that is what turns "I stopped reading" into "I am done", and it
    /// is the difference between a provider that keeps working and one
    /// that is told.
    ///
    /// It takes a [`Handle`] rather than a frame on purpose: by the time
    /// a destructor runs, the scope number may belong to somebody else,
    /// and only a handle can check. Nothing is sent for a scope that has
    /// closed.
    ///
    /// Nothing is sent for a stream that ended either — an ending is
    /// the provider saying it is finished, and there is nothing left to
    /// ask of it.
    ///
    /// # Not every endpoint has one
    ///
    /// Most do not, and it would mean nothing if they did. A listing
    /// answers and finishes; there is no moment at which a caller could
    /// usefully say stop. Leave this off and dropping a stream stops
    /// only the reading, which is all it has ever done.
    pub fn stop_with(
        mut self,
        handle: Handle,
        scope: u32,
        payload: Bytes,
    ) -> Self {
        self.0.stop_with(handle, scope, payload);
        self
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
