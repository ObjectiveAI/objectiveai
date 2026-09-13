//! The changes on the tree, for as long as the watch lasts.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::UnboundedReceiver;

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
/// receiver is unbounded. Holding one unread would mean a provider that
/// opened channels into it grew a queue nobody would ever read, for as
/// long as the watch lasted.
///
/// So [`execute`](super::execute) drops it and a stray channel request
/// dead-letters, which is the rule
/// [`Scope`](crate::client::scope::Scope) states about itself: drop
/// what you are not going to read.
///
/// # Reading is not optional
///
/// The queue is unbounded, so a watch nobody reads stalls nothing — it
/// grows, at whatever rate the tree is changing. A directory under a
/// build sends thousands of frames a second, and every one of them is
/// kept until somebody takes it or this is dropped.
///
/// That obligation gets sharper as a [`Stream`], not softer. A
/// `select!` is exactly where a stream someone means to ignore ends up,
/// and one parked there that rarely wins is one accumulating a tree's
/// worth of changes nobody asked to keep.
///
/// # Dropping it says nothing
///
/// Which it used to. Ending a watch is
/// [`ExecuteHandle::disconnect`](super::ExecuteHandle::disconnect) now,
/// and this type carries nothing to send and no way to send it.
///
/// So dropping this stops a caller reading and stops nothing else. The
/// provider goes on reporting a tree into a queue the router discards,
/// until somebody disconnects or the connection goes.
#[must_use = "a watch that is not polled grows a queue nobody reads"]
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
    response_receiver: Option<UnboundedReceiver<Bytes>>,
}

impl ExecuteStream {
    /// Take the responses, from the [`execute`](super::execute) that
    /// has them.
    ///
    /// Not public. A watch exists because a request went out, so the
    /// only thing that can honestly make one of these is the thing that
    /// sent it.
    pub(super) fn new(response_receiver: UnboundedReceiver<Bytes>) -> Self {
        ExecuteStream {
            response_receiver: Some(response_receiver),
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
        // One channel end, and it is `Unpin`, so this never has to
        // project.
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
