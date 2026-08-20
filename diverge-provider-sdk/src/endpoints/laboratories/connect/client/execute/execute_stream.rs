//! The container's filesystem, for as long as the connection lasts.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::response;
use crate::decode::Decode;
use crate::frame;
use crate::shared::error::Error;
use crate::shared::filetree;

/// What the container's filesystem is doing, change by change.
///
/// Half of what [`execute`](super::execute) gives back. The other half
/// is an [`ExecuteHandle`](super::ExecuteHandle), and the two do
/// opposite jobs: this is everything the provider says without being
/// asked, and that is how a connector asks for anything.
///
/// The item is [`filetree::response::Frame`] — one change at a time
/// over the same tree a runner sees. What the sequence means is
/// [`filetree`]'s to say.
///
/// # It is the whole of what a connector is told
///
/// Not how many others are attached, not who they are, not when one
/// arrives or leaves. The container is what a connector joined, and
/// the guest list is the runner's business — a
/// [`run`](crate::endpoints::laboratories::run) hears about departures
/// because only a runner has the names to make sense of them.
///
/// # Dropping it takes nothing with it
///
/// Unlike a [`watch`](crate::endpoints::volumes::watch), where the
/// stream IS the subscription and dropping it says stop. Here the
/// connection is the
/// [`ExecuteHandle`](super::ExecuteHandle)'s, so a connector that
/// wants files rather than filesystem news drops this and goes on
/// working. The router's sends into it then fail, which it ignores, and
/// the queue is freed.
///
/// What that costs is the ending: with this gone, nothing observes the
/// scope finishing or the provider's error. A connector that wants to
/// hear a refusal keeps this at least until it has.
///
/// # Zero or more changes, then one ending
///
/// It yields zero or more [`Ok`], and then either ends or yields
/// exactly one [`Err`] and ends. Every error is terminal, which is what
/// makes the type explainable in one line and what [`FusedStream`] then
/// reports honestly.
///
/// [`None`] is the scope finishing.
/// [`ExecuteStreamError::Closed`] is the connection going away
/// underneath it, and the two are worth telling apart — one says the
/// connection is over, the other says nothing at all about whether the
/// container still exists.
///
/// There is no timeout, here or anywhere else in this protocol. A
/// container whose filesystem is not changing is a container that says
/// nothing for as long as that lasts.
#[must_use = "a filetree that is not polled grows a queue nobody reads"]
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
    /// [`poll_recv`](UnboundedReceiver::poll_recv) latches [`None`]
    /// forever once the senders are gone, so a stream that reported
    /// that as an error each time it saw it would report it without
    /// end. That is the obvious job.
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
    /// Not public. A connection exists because a request went out, so
    /// the only thing that can honestly make one of these is the thing
    /// that sent it.
    ///
    /// It takes one field and nothing else, which is the difference
    /// between this and a [`watch`](crate::endpoints::volumes::watch)'s
    /// stream: that one carries a handle, a scope and a frame so its
    /// destructor can say stop. This says nothing when it goes.
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
            // The finish, which is the connection ending as it should.
            frame::server::ServerFrame::ResponseFinish { .. } => {
                this.response_receiver = None;
                return Poll::Ready(None);
            }
            _ => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(ExecuteStreamError::Misrouted)));
            }
        };
        Poll::Ready(Some(match response::Frame::decode(payload) {
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

/// Whether the connection's stream is over.
///
/// Free to answer, because the terminal state is a field rather than
/// something to work out — and worth answering, because this is exactly
/// the stream that ends up in a `select!` beside a connector's own
/// work, which wants a fused one.
impl FusedStream for ExecuteStream {
    fn is_terminated(&self) -> bool {
        self.response_receiver.is_none()
    }
}

/// A connection whose stream stopped without ending.
///
/// None of these is the connection closing as it should. That is
/// [`None`] from the stream, and the difference is the whole reason
/// this type exists: a scope that finished said the connection is over,
/// and a scope that broke said nothing about whether the container is.
///
/// Every one of them is the last thing the stream yields. A connector
/// that wants to go on after one connects again; there is nothing to
/// resume, because a filetree stream opens with a snapshot and half of
/// one is not a smaller tree.
///
/// It is flat, and beside [`ExecuteError`](super::ExecuteError) rather
/// than inside it: one is a connection that never opened, this is one
/// that opened and then stopped without closing.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The connection ended underneath the scope.
    ///
    /// The changes stopped without a finish, so whether the container
    /// is still running is unknown — not settled either way.
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
    /// Neither a response nor the finish. Unreachable for the same
    /// reason as [`Frame`](Self::Frame): a router matches on the
    /// frame's type and gives each arm exactly one destination, so a
    /// scope's receiver only ever sees types `2` and `3`.
    ///
    /// Reported rather than treated as the end, because that would be
    /// the one confusion this protocol works hardest to prevent: a
    /// connection that BROKE arriving as one that CLOSED.
    Misrouted,
    /// The response frame did not decode.
    Response(response::FrameError),
    /// The provider could not open the connection, and said so.
    ///
    /// Which is usually not the provider's own refusal: a connector's
    /// authorization is relayed to whoever holds the container's run
    /// scope, and this is what a runner saying no comes back as. The
    /// laboratory not being there arrives the same way.
    ///
    /// It is the one response variant that ends the scope rather than
    /// adding to it, so it is an error here rather than an item.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => {
                f.write_str("connection ended in the middle of a connection")
            }
            ExecuteStreamError::Frame(error) => {
                write!(f, "connection frame did not decode: {error}")
            }
            ExecuteStreamError::Misrouted => f.write_str(
                "a frame arrived that does not belong on a connection",
            ),
            ExecuteStreamError::Response(error) => {
                write!(f, "connection frame did not parse: {error}")
            }
            ExecuteStreamError::Provider(_) => {
                f.write_str("the connection was not opened")
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
