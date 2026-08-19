//! The loop both response streams run.

use std::fmt;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use tokio::sync::mpsc::Receiver;

use super::response_stream_error::ResponseStreamError;
use crate::frame;

/// Which pair of frames a stream is peeling.
///
/// The only thing that differs between a scope's responses and a
/// channel's. Both arrive on a [`Receiver<Bytes>`] the router filled,
/// both end at a finish, and both carry a payload nobody here can read
/// — so the loop is written once and this says which two variants it
/// is looking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// [`Response`](crate::frame::server::ServerFrame::Response), ended
    /// by
    /// [`ResponseFinish`](crate::frame::server::ServerFrame::ResponseFinish).
    Scope,
    /// [`ChannelResponse`](crate::frame::server::ServerFrame::ChannelResponse),
    /// ended by
    /// [`ChannelResponseFinish`](crate::frame::server::ServerFrame::ChannelResponseFinish).
    Channel,
}

/// What a response stream is, under either of its two names.
///
/// Not public, and not because it is unfinished: the kind belongs in
/// the TYPE, so that a stream over a channel's receiver cannot be built
/// as a scope's by passing the wrong argument. The two public wrappers
/// give it that, and this keeps the loop from being written twice.
pub(super) struct ResponseStream<T, E> {
    /// The frames, until there are no more.
    ///
    /// [`None`] once the stream has ended, which is the terminal state
    /// and the whole of it.
    ///
    /// # It does two jobs, and the second is the load-bearing one
    ///
    /// [`poll_recv`](Receiver::poll_recv) latches [`None`] forever once
    /// the senders are gone, so a stream that reported that as an error
    /// each time it saw it would report it without end. That is the
    /// obvious job.
    ///
    /// The other: after a decode failure the channel is still open and
    /// may still be full of frames. Nothing about the receiver stops a
    /// stream that has declared itself finished from going on to yield
    /// items, and this does.
    ///
    /// An [`Option`] rather than a flag beside a receiver, because
    /// taking it out is what a flag would only record: the queue is
    /// dropped the moment nothing will read it, so its frames are freed
    /// and the router's later sends fail at once instead of filling
    /// something nobody is holding.
    responses: Option<Receiver<Bytes>>,
    /// What turns one payload into one item.
    ///
    /// The only part of a response stream that belongs to an endpoint.
    /// It gets the payload with the envelope already off, and returns
    /// the item or the endpoint's own error — which is where a
    /// provider's error frame is folded in, since telling one from an
    /// answer is a thing only the endpoint knows how to do.
    ///
    /// # It takes owned [`Bytes`], not a slice
    ///
    /// So that an item can KEEP the payload without copying it. A
    /// channel carrying an image layer or a database connection wants
    /// to hand those bytes on as they are, and a slice would force a
    /// copy per frame to do it. What this hands over is a refcounted
    /// view of exactly the payload, which costs one increment.
    ///
    /// # A function pointer, not [`Decode`](crate::decode::Decode)
    ///
    /// Two reasons, and the second is the real one. A decoded value may
    /// borrow from the bytes it came from, and a stream item may not.
    ///
    /// And an endpoint's response frame is almost always an ANSWER or a
    /// failure, so the peel has to split those two apart — the failure
    /// becoming the endpoint's error rather than an item. A trait bound
    /// producing the frame itself would hand a caller a provider's "I
    /// stopped" as though it were something the provider said about the
    /// work.
    decode: fn(Bytes) -> Result<T, E>,
    /// Which two frames to look for. See [`Kind`].
    kind: Kind,
}

impl<T, E> ResponseStream<T, E> {
    /// Take a receiver and the thing that reads what comes out of it.
    pub(super) fn new(
        responses: Receiver<Bytes>,
        decode: fn(Bytes) -> Result<T, E>,
        kind: Kind,
    ) -> Self {
        ResponseStream {
            responses: Some(responses),
            decode,
            kind,
        }
    }

    /// Whether the stream is over.
    ///
    /// Free to answer, because the terminal state is a field rather
    /// than something to work out.
    pub(super) fn is_terminated(&self) -> bool {
        self.responses.is_none()
    }

    /// One frame, peeled.
    ///
    /// # Every error is the last item
    ///
    /// A stream yields zero or more [`Ok`], and then either ends or
    /// yields exactly one [`Err`] and ends.
    ///
    /// Hard-coded rather than offered, because the choice is already
    /// available where it belongs: a failure an endpoint can recover
    /// from goes inside `T`, and one it cannot goes in the decoder's
    /// [`Err`]. `T` is whatever the endpoint says, so an endpoint that
    /// wants to survive a bad frame yields an item that says so and
    /// keeps reading. Nothing here has to have an opinion about which
    /// failures are which.
    ///
    /// # A frame that does not belong is an error, not an ending
    ///
    /// Only the two variants this kind names should reach these
    /// receivers — a [`Router`](super::router::Router) matches on the
    /// frame's type and each arm has exactly one destination. But
    /// [`Scope`](super::scope::Scope) and
    /// [`Channel`](super::channel::Channel) are public and so are their
    /// receivers, so "cannot happen" is a fact about this crate's
    /// router rather than about the type.
    ///
    /// Treating an unfamiliar frame as the finish would be the one
    /// confusion this protocol works hardest to prevent: a stream that
    /// BROKE reported as a stream that ENDED. So it is reported.
    pub(super) fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<T, ResponseStreamError<E>>>> {
        let Some(responses) = self.responses.as_mut() else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(responses.poll_recv(cx)) else {
            self.responses = None;
            return Poll::Ready(Some(Err(ResponseStreamError::Closed)));
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => {
                self.responses = None;
                return Poll::Ready(Some(Err(ResponseStreamError::Frame(
                    error,
                ))));
            }
        };
        let payload = match (self.kind, &envelope) {
            (Kind::Scope, frame::server::ServerFrame::Response { payload, .. })
            | (
                Kind::Channel,
                frame::server::ServerFrame::ChannelResponse { payload, .. },
            ) => *payload,
            // The finish, which is the stream ending as it should.
            (Kind::Scope, frame::server::ServerFrame::ResponseFinish { .. })
            | (
                Kind::Channel,
                frame::server::ServerFrame::ChannelResponseFinish { .. },
            ) => {
                self.responses = None;
                return Poll::Ready(None);
            }
            _ => {
                self.responses = None;
                return Poll::Ready(Some(Err(ResponseStreamError::Misrouted)));
            }
        };
        // A refcounted view of exactly the payload, which is what lets
        // an item keep the bytes rather than copy them. The slice came
        // out of `bytes` a moment ago, so it is a subset of it.
        let payload = bytes.slice_ref(payload);
        Poll::Ready(Some(match (self.decode)(payload) {
            Ok(item) => Ok(item),
            Err(error) => {
                self.responses = None;
                Err(ResponseStreamError::Payload(error))
            }
        }))
    }
}

/// What can be shown of it, which is not the decoder.
///
/// Hand-written rather than derived, because the struct mentions `T`
/// and `E` only inside a function pointer — a derive would demand
/// [`Debug`] of both to print neither.
impl<T, E> fmt::Debug for ResponseStream<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseStream")
            .field("kind", &self.kind)
            .field("terminated", &self.is_terminated())
            .finish_non_exhaustive()
    }
}
