//! What a running laboratory says about itself.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::response;
use super::run_frame::RunFrame;
use crate::decode::Decode;
use crate::frame;
use crate::shared::error::Error;

/// A laboratory that is running, and what it reports.
///
/// Half of what [`execute`](super::execute) gives back. The other half
/// is an [`ExecuteHandle`](super::ExecuteHandle), and the two do
/// opposite jobs: this is everything the provider says without being
/// asked, and that is how a runner asks for anything.
///
/// The item is [`RunFrame`] — the container's id, changes on its
/// filesystem, and connectors leaving. See it for what may be assumed
/// about the order they arrive in, which is almost nothing.
///
/// # It says more than a connector is told
///
/// A [`connection`](crate::endpoints::laboratories::connect) hears only
/// about the filesystem. A runner hears about departures too, because a
/// runner is the party that authorized every arrival — it is the only
/// one holding names to make sense of them, and the only one that ever
/// agreed to the connectors in the first place.
///
/// # Zero or more reports, then one ending
///
/// It yields zero or more [`Ok`], and then either ends or yields
/// exactly one [`Err`] and ends. Every error is terminal, which is what
/// makes the type explainable in one line and what [`FusedStream`] then
/// reports honestly.
///
/// [`None`] is the scope finishing, which is the laboratory being over
/// — because the scope IS the container's life.
/// [`ExecuteStreamError::Closed`] is the connection going away
/// underneath it, and the two are worth telling apart: one says the
/// container is gone, the other says nothing about whether it is.
///
/// There is no timeout, here or anywhere else in this protocol. A
/// container nobody is touching reports nothing for as long as that
/// lasts.
///
/// # Dropping it does not stop the laboratory
///
/// The container's life belongs to the
/// [`ExecuteHandle`](super::ExecuteHandle), and
/// [`stop`](super::ExecuteHandle::stop) is what ends it. Dropping this
/// stops a runner seeing what the container is doing and stops nothing
/// else — the provider goes on reporting into a queue the router
/// discards.
///
/// Which is a real choice rather than a free one: with this gone, a
/// runner no longer hears departures, and it will not observe the
/// laboratory ending either.
#[must_use = "a laboratory that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ExecuteStream {
    /// The scope's responses, until there are no more.
    ///
    /// [`None`] once the stream has ended, which is the terminal state
    /// and the whole of it.
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
    /// reports, and this does.
    response_receiver: Option<UnboundedReceiver<Bytes>>,
}

impl ExecuteStream {
    /// Take the responses, from the [`execute`](super::execute) that
    /// has them.
    ///
    /// Not public. A laboratory exists because a request went out, so
    /// the only thing that can honestly make one of these is the thing
    /// that sent it.
    pub(super) fn new(response_receiver: UnboundedReceiver<Bytes>) -> Self {
        ExecuteStream {
            response_receiver: Some(response_receiver),
        }
    }
}

/// One report at a time, until there are no more.
///
/// See the type's own documentation for what ends it and what that
/// means.
impl Stream for ExecuteStream {
    type Item = Result<RunFrame, ExecuteStreamError>;

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
            // The finish, which is the laboratory ending as it should.
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
            Ok(response::Frame::Id(id)) => Ok(RunFrame::Id(id)),
            Ok(response::Frame::Filetree(frame)) => {
                Ok(RunFrame::Filetree(frame))
            }
            Ok(response::Frame::Disconnected(disconnected)) => {
                Ok(RunFrame::Disconnected(disconnected))
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

/// Whether the laboratory's stream is over.
///
/// Free to answer, because the terminal state is a field rather than
/// something to work out.
impl FusedStream for ExecuteStream {
    fn is_terminated(&self) -> bool {
        self.response_receiver.is_none()
    }
}

/// A laboratory whose stream stopped without ending.
///
/// None of these is the laboratory closing as it should. That is
/// [`None`] from the stream, and the difference is the whole reason
/// this type exists: a scope that finished said the container is gone,
/// and a scope that broke said nothing about whether it is.
///
/// Every one of them is the last thing the stream yields.
///
/// It is flat, and beside [`ExecuteError`](super::ExecuteError) rather
/// than inside it: one is a laboratory that never started, this is one
/// that started and then stopped without ending.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The connection ended underneath the scope.
    ///
    /// The reports stopped without a finish, so whether the container
    /// is still running is unknown — not settled either way. A provider
    /// that lost its runner will tear it down on its own, but nothing
    /// here saw that happen.
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
    /// Neither a response nor the finish. Reported rather than treated
    /// as the end, because that would be the one confusion this
    /// protocol works hardest to prevent: a laboratory that BROKE
    /// arriving as one that ENDED.
    Misrouted,
    /// The response frame did not decode.
    Response(response::FrameError),
    /// The provider could not run the laboratory, and said so.
    ///
    /// The image would not pull, the container would not start,
    /// whatever the provider knows. It is the one response variant that
    /// ends the scope rather than adding to it, so it is an error here
    /// rather than an item.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Closed => f.write_str(
                "the connection ended in the middle of a laboratory",
            ),
            ExecuteStreamError::Frame(error) => {
                write!(f, "laboratory frame did not decode: {error}")
            }
            ExecuteStreamError::Misrouted => f.write_str(
                "a frame arrived that does not belong on a laboratory",
            ),
            ExecuteStreamError::Response(error) => {
                write!(f, "laboratory frame did not parse: {error}")
            }
            ExecuteStreamError::Provider(_) => {
                f.write_str("the laboratory was not run")
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
