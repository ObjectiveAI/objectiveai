//! A file coming out of the container, piece by piece.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::channel_response;
use crate::decode::Decode;
use crate::frame;
use crate::shared::error::Error;

/// One file, arriving.
///
/// What [`read`](super::ExecuteHandle::read) gives back. The request
/// has gone out and the router is already putting the answer where this
/// will find it.
///
/// The item is [`Bytes`] — a piece of the file, refcounted out of the
/// frame it arrived in rather than copied. How many pieces there are
/// and how large each is belongs to the provider; a reader that wants
/// the whole file concatenates them, and one that is piping it
/// somewhere writes each as it comes.
///
/// # Zero or more pieces, then one ending
///
/// It yields zero or more [`Ok`], and then either ends or yields
/// exactly one [`Err`] and ends. Every error is terminal, which is what
/// makes the type explainable in one line and what [`FusedStream`] then
/// reports honestly.
///
/// [`None`] is the file, complete. An empty file is a stream that ends
/// having yielded nothing, which is the same shape as a file of zero
/// bytes because that is what it is.
///
/// [`ReadStreamError::Provider`] is the provider saying the file was
/// not read, or not all of it — and the distinction between those two
/// is not one it draws, which is why a partial read cannot be told from
/// a refused one. What arrived before the error arrived, and whether it
/// is a prefix of the file or the whole of it is unknowable from here.
///
/// # Dropping it does not cancel the read
///
/// There is no frame for that. A connector that stops reading leaves
/// the provider sending into a queue nobody holds, which the router
/// discards — so the bytes stop being delivered but do not stop being
/// produced, and the channel stays open until the provider finishes it.
///
/// What that costs is one channel's worth of work at the provider,
/// until the file runs out. It is the same trade every unread stream in
/// this crate makes, and the remedy is the same: drop what you are not
/// going to read, and accept that the far end finds out late.
#[must_use = "a read that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct ReadStream {
    /// The channel's answer, until there is no more of it.
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
    /// pieces of a file it has already said it could not read, and this
    /// does.
    response_receiver: Option<UnboundedReceiver<Bytes>>,
}

impl ReadStream {
    /// Take the channel's answers, from the
    /// [`read`](super::ExecuteHandle::read) that asked for them.
    ///
    /// Not public. A read exists because a request went out, so the
    /// only thing that can honestly make one of these is the thing that
    /// sent it.
    pub(super) fn new(response_receiver: UnboundedReceiver<Bytes>) -> Self {
        ReadStream {
            response_receiver: Some(response_receiver),
        }
    }
}

/// One piece of the file at a time, until there are no more.
///
/// See the type's own documentation for what ends it and what that
/// means.
impl Stream for ReadStream {
    type Item = Result<Bytes, ReadStreamError>;

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
            return Poll::Ready(Some(Err(ReadStreamError::Closed)));
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(ReadStreamError::Frame(error))));
            }
        };
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => {
                payload
            }
            // The finish, which is the file ending as it should.
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                this.response_receiver = None;
                return Poll::Ready(None);
            }
            _ => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(ReadStreamError::Misrouted)));
            }
        };
        Poll::Ready(Some(
            match channel_response::read::Frame::decode(payload) {
                // A refcounted view of exactly the body, which is what
                // lets a piece of the file be kept rather than copied.
                // The slice came out of `bytes` a moment ago, so it is
                // a subset of it.
                Ok(channel_response::read::Frame::Body(body)) => {
                    Ok(bytes.slice_ref(body.0))
                }
                Ok(channel_response::read::Frame::Error(error)) => {
                    this.response_receiver = None;
                    Err(ReadStreamError::Provider(error))
                }
                Err(error) => {
                    this.response_receiver = None;
                    Err(ReadStreamError::Response(error))
                }
            },
        ))
    }
}

/// Whether the read is over.
///
/// Free to answer, because the terminal state is a field rather than
/// something to work out.
impl FusedStream for ReadStream {
    fn is_terminated(&self) -> bool {
        self.response_receiver.is_none()
    }
}

/// A read that stopped without ending.
///
/// None of these is the file arriving in full. That is [`None`] from
/// the stream, and the difference is the whole reason this type exists:
/// a read that ended delivered the file, and a read that broke
/// delivered a prefix of one and cannot say how much of it.
///
/// It is flat, and beside [`ReadError`](super::ReadError) rather than
/// inside it: one is a read that never started, this is one that
/// started and then stopped without ending.
#[derive(Debug)]
pub enum ReadStreamError {
    /// The connection ended mid-file.
    ///
    /// The pieces stopped without a finish, so what arrived is a prefix
    /// of the file and there is no way to learn how much was missing.
    Closed,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own
    /// [`Router`](crate::client::router::Router), which decodes the
    /// same bytes before forwarding them and discards what will not
    /// parse.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on a channel's answer.
    ///
    /// Neither a response nor the finish. Reported rather than treated
    /// as the end, because that would be the one confusion this
    /// protocol works hardest to prevent: a read that BROKE arriving as
    /// a file that was shorter than it is.
    Misrouted,
    /// The answer did not parse.
    Response(channel_response::read::FrameError),
    /// The provider could not read the file, or not all of it.
    ///
    /// Which of the two is not something it says. A caller that has
    /// already taken pieces knows only that the file is longer than
    /// what it holds.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for ReadStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadStreamError::Closed => {
                f.write_str("the connection ended in the middle of a read")
            }
            ReadStreamError::Frame(error) => {
                write!(f, "read frame did not decode: {error}")
            }
            ReadStreamError::Misrouted => {
                f.write_str("a frame arrived that does not belong on a read")
            }
            ReadStreamError::Response(error) => {
                write!(f, "read frame did not parse: {error}")
            }
            ReadStreamError::Provider(_) => {
                f.write_str("the file was not read")
            }
        }
    }
}

impl std::error::Error for ReadStreamError {
    /// [`Provider`](ReadStreamError::Provider) has no source, because
    /// what it carries is not a Rust error and deliberately does not
    /// implement one — see
    /// [`shared::error::Error`](crate::shared::error::Error).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadStreamError::Frame(error) => Some(error),
            ReadStreamError::Response(error) => Some(error),
            ReadStreamError::Closed
            | ReadStreamError::Misrouted
            | ReadStreamError::Provider(_) => None,
        }
    }
}
