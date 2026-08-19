//! The changes on the tree, for as long as the watch lasts.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::Receiver;

use crate::client::handle::Handle;
use crate::decode::Decode;
use crate::endpoints::volumes::watch::server::response;
use crate::frame;
use crate::shared::error::Error;
use crate::shared::filetree;

/// A watch that is running, and the changes coming out of it.
///
/// What [`execute`](super::execute) gives back. The request has gone
/// out and the router is already putting frames where this will find
/// them.
///
/// The item is [`filetree::response::Frame`], the SHARED one — a
/// snapshot and then changes to it. Taking this endpoint's own envelope
/// off is most of what this type is for: a caller folding a tree wants
/// the change, not the news that a change is what this is. What the
/// sequence means, and what a reader has to hold to make sense of it,
/// is [`filetree`]'s to say.
///
/// # Zero or more changes, then one ending
///
/// It yields zero or more [`Ok`], and then either ends or yields
/// exactly one [`Err`] and ends. Every error is terminal, which is what
/// makes the type explainable in one line and what [`FusedStream`] then
/// reports honestly.
///
/// [`None`] is the provider finishing as it should.
/// [`ExecuteStreamError::Closed`] is the connection going away
/// mid-watch, and the two are worth telling apart — one says the tree
/// is no longer being reported, the other says nothing at all, so what
/// the tree looks like now is unknown rather than unchanged.
///
/// There is no timeout, here or anywhere else in this protocol. A tree
/// nobody is touching is a watch that says nothing for as long as that
/// lasts, and a quiet stream is not a finished one.
///
/// # It keeps the responses and nothing else of the scope
///
/// A [`Scope`](crate::client::scope::Scope) carries a second receiver,
/// for channels a provider opens inside it. Nothing is supposed to open
/// one inside a watch — but nothing forbids it either, and that
/// receiver is bounded at `1` while the router AWAITS it. Holding one
/// unread would mean a provider that opened two channels blocked the
/// router forever, stalling every scope on the connection.
///
/// So [`execute`](super::execute) drops it and a stray channel request
/// dead-letters, which is the rule
/// [`Scope`](crate::client::scope::Scope) states about itself: drop
/// what you are not going to read.
///
/// # Reading is not optional
///
/// The queue is bounded, at whatever depth
/// [`execute`](super::execute) was given. A watch nobody reads stops
/// the router that many frames later, and stopping the router stops
/// every scope on the connection rather than only this one.
///
/// That obligation gets sharper as a [`Stream`], not softer. A
/// `select!` is exactly where a stream someone means to ignore ends up,
/// and one parked there that rarely wins stalls the connection once its
/// capacity fills.
///
/// # Dropping it disconnects the watch
///
/// There is still no frame for cancelling a SCOPE — a client opens one
/// and a server ends one, and nothing in
/// [`ClientFrame`](crate::frame::client::ClientFrame) says stop at that
/// level. What a watch has instead is a
/// [`channel_request`](super::channel_request) that means it,
/// and this carries one and sends it on the way out.
///
/// Which makes dropping the ordinary way to be done with a watch rather
/// than a way to abandon one: the provider is told instead of left
/// walking a tree nobody is listening about, and the scope number comes
/// back.
///
/// Best-effort in one direction only. A destructor has nowhere to
/// report a failure and nothing to await it, so a request that cannot
/// go — no runtime on the thread, a connection already gone — leaves
/// things as they were before any of this existed. It never sends the
/// wrong one: see [`drop`](Self::drop).
#[must_use = "a watch that is not polled stalls every scope on the connection"]
#[derive(Debug)]
pub struct ExecuteStream {
    /// The scope's responses, until there are no more.
    ///
    /// [`None`] once the stream has ended, which is the terminal state
    /// and the whole of it. Named for the
    /// [`Scope`](crate::client::scope::Scope) field it came out of,
    /// because that is exactly what it is.
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
    /// changes, and this does.
    ///
    /// An [`Option`] rather than a flag beside a receiver, because
    /// taking it out is what a flag would only record: the queue is
    /// dropped the moment nothing will read it, so its frames are freed
    /// and the router's later sends fail at once instead of filling
    /// something nobody is holding.
    response_receiver: Option<Receiver<Bytes>>,
    /// What the disconnect is sent over.
    ///
    /// A [`Handle`] rather than a pre-encoded frame, and the difference
    /// matters: a frame carries a scope number it cannot re-check, and
    /// by the time a destructor runs that number may belong to somebody
    /// else. A scope ends, the router says so, the next mint hands the
    /// same number out again — and a frame would then disconnect a
    /// watch the caller had just started.
    ///
    /// [`send_channel_request`](Handle::send_channel_request) takes
    /// back what the router has closed and then looks the scope up, so
    /// a scope that is gone sends nothing. That check is the guard, and
    /// only a handle has it.
    handle: Handle,
    /// The scope this watch is running in, as this end numbered it.
    ///
    /// Kept only to say which scope the disconnect is for. Nothing else
    /// here needs it — the router already sorted the frames by it.
    scope: u32,
    /// The disconnect, encoded and ready.
    ///
    /// Built once by [`execute`](super::execute) through
    /// [`channel_request::Frame`](super::channel_request::Frame)
    /// rather than written as the byte it happens to be. A destructor
    /// is a poor place to be encoding anything, and what a disconnect
    /// looks like on the wire is not this type's to know.
    disconnect_request: Bytes,
}

impl ExecuteStream {
    /// Take the pieces, from the [`execute`](super::execute) that has
    /// them.
    ///
    /// Not public. A watch exists because a request went out, so the
    /// only thing that can honestly make one of these is the thing that
    /// sent it.
    pub(super) fn new(
        response_receiver: Receiver<Bytes>,
        handle: Handle,
        scope: u32,
        disconnect_request: Bytes,
    ) -> Self {
        ExecuteStream {
            response_receiver: Some(response_receiver),
            handle,
            scope,
            disconnect_request,
        }
    }
}

/// One change on the tree at a time, until there are no more.
///
/// See the type's own documentation for what ends it and what that
/// means.
impl Stream for ExecuteStream {
    type Item = Result<filetree::response::Frame, ExecuteStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // A channel end, a handle, a number and some bytes — all
        // `Unpin`, so this never has to project.
        let this = self.get_mut();
        let Some(responses) = this.response_receiver.as_mut() else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(responses.poll_recv(cx)) else {
            this.response_receiver = None;
            return Poll::Ready(Some(Err(ExecuteStreamError::Closed)));
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(ExecuteStreamError::Frame(
                    error,
                ))));
            }
        };
        let payload = match envelope {
            frame::server::ServerFrame::Response { payload, .. } => payload,
            // The finish, which is the watch ending as it should.
            frame::server::ServerFrame::ResponseFinish { .. } => {
                this.response_receiver = None;
                return Poll::Ready(None);
            }
            _ => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(
                    ExecuteStreamError::Misrouted,
                )));
            }
        };
        // A refcounted view of exactly the payload, which is what lets
        // a change keep the bytes rather than copy them. The slice came
        // out of `bytes` a moment ago, so it is a subset of it.
        let payload = bytes.slice_ref(payload);
        Poll::Ready(Some(match response::Frame::decode(&payload) {
            Ok(response::Frame::Filetree(frame)) => Ok(frame),
            Ok(response::Frame::Error(error)) => {
                this.response_receiver = None;
                Err(ExecuteStreamError::Provider(error))
            }
            Err(error) => {
                this.response_receiver = None;
                Err(ExecuteStreamError::Response(error))
            }
        }))
    }
}

/// Whether the watch is over.
///
/// Free to answer, because the terminal state is a field rather than
/// something to work out — and worth answering, because a watch is
/// exactly the stream that ends up in a `select!`, which wants a fused
/// one.
impl FusedStream for ExecuteStream {
    fn is_terminated(&self) -> bool {
        self.response_receiver.is_none()
    }
}

/// Tell the provider to stop watching.
///
/// # It spawns rather than sends
///
/// A destructor cannot await, and writing a frame means locking a
/// connection and waiting on a socket. What it can do is hand the whole
/// thing to a runtime and return, which is all this does.
///
/// [`try_current`](tokio::runtime::Handle::try_current) rather than
/// [`tokio::spawn`], because `spawn` PANICS outside a runtime and a
/// destructor is the worst place in a program to do that. No runtime
/// means no disconnect, which leaves things exactly as they were before
/// this existed.
///
/// # It says nothing about a watch that ended
///
/// A watch that has ended has nothing to stop — and worse, its scope
/// number may since have been handed out again, so a late disconnect
/// could end somebody else's work. The terminal state is already a
/// field, so the check is free.
///
/// That covers every ending: a finish, a closed connection, and each of
/// the errors. What it does not cover is a finish sitting unread in the
/// queue when a caller drops, and that window is closed one level down
/// —
/// [`send_channel_request`](Handle::send_channel_request) looks the
/// scope up after taking back what the router has closed, and a scope
/// that is gone sends nothing.
///
/// # The answer is dropped
///
/// Nothing answers a disconnect; what answers it is the scope's own
/// finish. So the channel this opens is abandoned immediately, and its
/// entry in the router lingers until the scope closes — which is the
/// thing the disconnect is provoking.
impl Drop for ExecuteStream {
    fn drop(&mut self) {
        if self.is_terminated() {
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let handle = self.handle.clone();
        let scope = self.scope;
        let disconnect_request = self.disconnect_request.clone();
        runtime.spawn(async move {
            let _ = handle
                .send_channel_request(scope, &disconnect_request, 1)
                .await;
        });
    }
}

/// A watch that stopped without ending.
///
/// None of these is the watch finishing. That is [`None`] from the
/// stream, and the difference is the whole reason this type exists: a
/// watch that ended told a caller the tree is no longer being reported,
/// and a watch that broke told it nothing.
///
/// Every one of them is the last thing the stream yields, and they are
/// terminal for one underlying reason: a filetree is a FOLD. The frames
/// apply to a tree the reader is keeping, so a gap in them leaves that
/// tree permanently wrong with no way to notice. Reading on would be
/// applying changes to something known to be broken. The recovery is a
/// new watch and a fresh snapshot, which is cheap.
///
/// It is flat, and beside
/// [`ExecuteError`](super::ExecuteError) rather than inside it: one is
/// a watch that never started, this is a watch that started and then
/// stopped without ending.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The connection ended mid-watch.
    ///
    /// The frames stopped without a finish, so what the tree looks like
    /// now is unknown — not unchanged.
    Closed,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own
    /// [`Router`](crate::client::router::Router), which decodes the
    /// same bytes before forwarding them and discards what will not
    /// parse. It is here because
    /// [`Scope`](crate::client::scope::Scope) is public and its
    /// receiver could be fed by something else.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on a scope's stream.
    ///
    /// Neither a response nor the finish — a channel's response, an
    /// auth frame, anything. Unreachable for the same reason as
    /// [`Frame`](Self::Frame): a router matches on the frame's type and
    /// gives each arm exactly one destination, so a scope's receiver
    /// only ever sees types `2` and `3`.
    ///
    /// Reported rather than treated as the end, because that would be
    /// the one confusion this protocol works hardest to prevent: a
    /// watch that BROKE arriving as a watch that ENDED.
    Misrouted,
    /// The response frame did not parse.
    Response(response::FrameDecodeError),
    /// The provider stopped watching, and said so.
    ///
    /// An answer of a kind — the provider is saying the watch is over
    /// and why — but not a change to the tree, which is why it is not
    /// the stream simply ending.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => {
                f.write_str("connection ended in the middle of a watch")
            }
            ExecuteStreamError::Frame(error) => {
                write!(f, "watch frame did not decode: {error}")
            }
            ExecuteStreamError::Misrouted => f.write_str(
                "a frame arrived that does not belong on a watch",
            ),
            ExecuteStreamError::Response(error) => {
                write!(f, "watch frame did not parse: {error}")
            }
            ExecuteStreamError::Provider(_) => {
                f.write_str("the provider stopped watching")
            }
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    /// [`Provider`](ExecuteStreamError::Provider) has no source,
    /// because what it carries is not a Rust error and deliberately
    /// does not implement one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Response(error) => Some(error),
            ExecuteStreamError::Closed
            | ExecuteStreamError::Misrouted
            | ExecuteStreamError::Provider(_) => None,
        }
    }
}
