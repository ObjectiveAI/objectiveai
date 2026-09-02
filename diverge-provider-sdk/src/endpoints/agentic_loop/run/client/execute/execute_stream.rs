//! The chunks of a loop, for as long as it runs.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

use crate::decode::Decode;
use crate::endpoints::agentic_loop::run::server::response;
use crate::frame;
use crate::shared::error::Error;

/// A loop that is running, and the chunks coming out of it.
///
/// What [`execute`](super::execute) gives back. The request has gone
/// out and the router is already putting frames where this will find
/// them.
///
/// The item is an [`ExecuteStreamItem`]: an
/// [`AgenticLoopChunk`](response::AgenticLoopChunk) — one event at a
/// time, rather than a partially-filled record of everything that
/// could have happened — or, at the close, a piece of the
/// continuation. What the sequence means is [`response`]'s to say.
///
/// # It is not the only thing running
///
/// A loop is the one endpoint so far where the provider asks the caller
/// for things WHILE answering it: an agent's tool calls come back out
/// as MCP requests, because the MCP servers live with the caller.
/// [`execute`](super::execute) puts a task on that, and this holds it.
///
/// So a caller reads chunks and nothing else. The proxying happens
/// beside it, at its own pace, and neither waits on the other — which
/// is the point. A caller that stops to think about a chunk would
/// otherwise be a caller that has stopped answering the agent's tools.
///
/// # Zero or more chunks, the closer, then one ending
///
/// It yields zero or more [`Ok`] — chunks, and then, if the provider
/// issued one, the continuation in pieces the caller KEEPS APART, in
/// order, the finish saying the sequence is whole — and then either
/// ends or yields exactly one
/// [`Err`] and ends. Every error is terminal, which is what makes the
/// type explainable in one line and what [`FusedStream`] then reports
/// honestly. The continuation is not terminal: only the finish is.
///
/// [`None`] is the loop finishing as it should.
/// [`ExecuteStreamError::Closed`] is the connection going away
/// mid-run, and the two are worth telling apart — one says the work is
/// done, the other says nothing at all about whether it was.
///
/// There is no timeout, here or anywhere else in this protocol. An
/// agent that is thinking is a loop that says nothing for as long as
/// that lasts, and a quiet stream is not a finished one.
///
/// # Dropping it stops you reading, and the proxying
///
/// The task answering the agent's MCP requests is aborted, because
/// serving tools for a loop nobody is listening to is work with no
/// reader.
///
/// What it does NOT do is stop the loop. There is no frame for
/// cancelling a scope, and unlike a
/// [`watch`](crate::endpoints::volumes::watch) this endpoint defines no
/// channel request that means stop — so a provider goes on running the
/// agent, and finds its tool calls unanswered. That is a gap in the
/// protocol rather than in this type, and it is the strongest argument
/// the loop has for one.
#[must_use = "a loop that is not polled grows a queue nobody reads"]
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
    /// chunks, and this does.
    response_receiver: Option<UnboundedReceiver<Bytes>>,
    /// The task answering the agent's MCP requests.
    ///
    /// Held only to end it. Nothing here waits on it or reads what it
    /// returns — it ends on its own when the scope closes, because the
    /// receiver it is reading closes with everything else under a
    /// finished scope.
    ///
    /// Aborting rather than detaching, so that dropping this stops the
    /// proxying too. It can land mid-answer, leaving a channel that has
    /// been answered in part and never finished — which is untidy and
    /// is also exactly what the far end would see from a caller that
    /// had crashed. The scope is being abandoned either way.
    proxying: JoinHandle<()>,
}

impl ExecuteStream {
    /// Take the responses, and the task that answers alongside them.
    ///
    /// Not public. A loop exists because a request went out, so the
    /// only thing that can honestly make one of these is the thing that
    /// sent it.
    pub(super) fn new(
        response_receiver: UnboundedReceiver<Bytes>,
        proxying: JoinHandle<()>,
    ) -> Self {
        ExecuteStream {
            response_receiver: Some(response_receiver),
            proxying,
        }
    }
}

/// One item of a running loop: a chunk, or a piece of its closer.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecuteStreamItem {
    /// One event of the loop. See
    /// [`AgenticLoopChunk`](response::AgenticLoopChunk).
    Chunk(response::AgenticLoopChunk),
    /// One piece of the continuation, the run's closer: raw bytes,
    /// the provider's own opaque state. Append every piece in order;
    /// the stream ending says the whole is whole. A caller keeps it
    /// and answers the next run's continuation fetch with it.
    ///
    /// Owned without a copy — a view onto the frame's own buffer.
    Continuation(Bytes),
}

/// One item at a time, until there are no more.
///
/// See the type's own documentation for what ends it and what that
/// means.
impl Stream for ExecuteStream {
    type Item = Result<ExecuteStreamItem, ExecuteStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // A channel end and a join handle — both `Unpin`, so this never
        // has to project.
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
            // The finish, which is the loop ending as it should.
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
            Ok(response::Frame::Chunk(chunk)) => {
                Ok(ExecuteStreamItem::Chunk(chunk))
            }
            // A view onto `bytes`, not a copy: `body` is a slice of
            // the buffer this frame arrived in.
            Ok(response::Frame::Continuation(body)) => {
                Ok(ExecuteStreamItem::Continuation(bytes.slice_ref(body)))
            }
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

/// Whether the loop is over.
///
/// Free to answer, because the terminal state is a field rather than
/// something to work out — and worth answering, because a loop is
/// exactly the stream that ends up in a `select!`, which wants a fused
/// one.
impl FusedStream for ExecuteStream {
    fn is_terminated(&self) -> bool {
        self.response_receiver.is_none()
    }
}

/// Stop answering the agent's tools.
///
/// Unconditional, unlike a [`watch`](crate::endpoints::volumes::watch)'s
/// drop: there is nothing to send and nothing to check, only a task
/// that has no reason to go on. A loop that has ended has already
/// closed the receiver that task is reading, so aborting it then is a
/// no-op.
impl Drop for ExecuteStream {
    fn drop(&mut self) {
        self.proxying.abort();
    }
}

/// A loop that stopped without ending.
///
/// None of these is the loop finishing. That is [`None`] from the
/// stream, and the difference is the whole reason this type exists: a
/// loop that ended told a caller the work is done, and a loop that
/// broke told it nothing about whether it was.
///
/// Every one of them is the last thing the stream yields. A caller that
/// wants to go on after one starts another loop; there is nothing to
/// resume, because a chunk stream is a sequence and half of one is not
/// a shorter one.
///
/// It is flat, and beside [`ExecuteError`](super::ExecuteError) rather
/// than inside it: one is a loop that never started, this is a loop
/// that started and then stopped without ending.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The connection ended mid-run.
    ///
    /// The chunks stopped without a finish, so whether the agent
    /// finished its work is unknown — not settled either way.
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
    /// the one confusion this protocol works hardest to prevent: a loop
    /// that BROKE arriving as a loop that ENDED.
    Misrouted,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// The provider could not go on, and said so.
    ///
    /// Distinct from a
    /// [`NotificationChunk`](response::NotificationChunk) that happens
    /// to be fatal: that is the loop reporting on itself and still
    /// owning the scope, and this is the scope ending.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => {
                f.write_str("connection ended in the middle of a loop")
            }
            ExecuteStreamError::Frame(error) => {
                write!(f, "loop frame did not decode: {error}")
            }
            ExecuteStreamError::Misrouted => {
                f.write_str("a frame arrived that does not belong on a loop")
            }
            ExecuteStreamError::Response(error) => {
                write!(f, "loop frame did not parse: {error}")
            }
            ExecuteStreamError::Provider(_) => {
                f.write_str("the provider could not go on with the loop")
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
