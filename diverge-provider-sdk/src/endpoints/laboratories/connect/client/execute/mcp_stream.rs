//! One MCP exchange, arriving.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::super::server::channel_response;
use super::mcp_frame::McpFrame;
use crate::decode::Decode;
use crate::frame;
use crate::shared::http::response;

/// One MCP answer out of the container, piece by piece.
///
/// What [`mcp`](super::ExecuteHandle::mcp) gives back. The request has
/// gone out and the router is already putting the answer where this
/// will find it.
///
/// The item is [`McpFrame`] — the head once, then as much body as there
/// turns out to be. It is the shape of the wire rather than a tidier
/// one on purpose: the two answers MCP gives, a JSON document and an
/// event stream held open for the session, are the same sequence here
/// and a caller decides which it is reading from `Content-Type` in the
/// head.
///
/// # There is no provider error, and there cannot be
///
/// Unlike [`read`](super::ExecuteHandle::read),
/// [`write`](super::ExecuteHandle::write) and
/// [`transfer`](super::ExecuteHandle::transfer), every one of which can
/// come back with the provider saying no. This channel has no error
/// frame, because the exchange is HTTP and HTTP already says how things
/// go wrong.
///
/// So a server that is not there answers `502` and a method that does
/// not exist answers `404`, and both of those arrive as a
/// [`Head`](McpFrame::Head) on a stream that is working perfectly.
/// Every variant of [`McpStreamError`] is this crate's plumbing
/// breaking, never a refusal — a caller that wants to know whether the
/// call succeeded reads the status.
///
/// # Zero or more items, then one ending
///
/// It yields items and then either ends or yields exactly one [`Err`]
/// and ends. Every error is terminal, which is what makes the type
/// explainable in one line and what [`FusedStream`] then reports
/// honestly.
///
/// [`None`] is the exchange finishing. A stream that ends having
/// yielded nothing at all is an exchange the container never answered
/// — which is what a server that is not listening looks like, since
/// there is no status to report when nothing produced one.
///
/// # There is no timeout
///
/// Here or anywhere else in this protocol. An event stream that is
/// quiet is a session with nothing to say, and a `POST` being thought
/// about is a channel that says nothing until it is thought about.
///
/// # Dropping it does not cancel the exchange
///
/// There is no frame for that. The provider goes on relaying into a
/// queue nobody holds, which the router discards — so the answer stops
/// being delivered and does not stop being produced.
///
/// For a JSON answer that is one document's worth of waste. For an
/// event stream held open for a session it is unbounded, and the remedy
/// is MCP's own rather than this protocol's: a `DELETE` carrying the
/// session id ends the session, and that is another
/// [`mcp`](super::ExecuteHandle::mcp) call like any other. There is no
/// channel-level cancel to go looking for.
#[must_use = "an exchange that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct McpStream {
    /// The channel's answer, until there is no more of it.
    ///
    /// [`None`] once the stream has ended, which is half the terminal
    /// state; [`headed`](Self::headed) is the other half.
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
    /// pieces of an answer it has already said it could not read, and
    /// this does.
    response_receiver: Option<UnboundedReceiver<Bytes>>,
    /// Whether the head has been seen.
    ///
    /// The protocol says a head comes once and never again, so this is
    /// what lets a second one be reported rather than passed on — and
    /// what lets a body arriving first be reported too, which is the
    /// same rule read from the other side.
    ///
    /// Nothing else in this crate tracks a position in a stream. It is
    /// here because this is the only stream whose items are not all
    /// alike.
    headed: bool,
}

impl McpStream {
    /// Take the channel's answers, from the
    /// [`mcp`](super::ExecuteHandle::mcp) that asked for them.
    ///
    /// Not public. An exchange exists because a request went out, so
    /// the only thing that can honestly make one of these is the thing
    /// that sent it.
    pub(super) fn new(response_receiver: UnboundedReceiver<Bytes>) -> Self {
        McpStream {
            response_receiver: Some(response_receiver),
            headed: false,
        }
    }
}

/// The head, then the body, until there is no more.
///
/// See the type's own documentation for what ends it and what that
/// means.
impl Stream for McpStream {
    type Item = Result<McpFrame, McpStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // A channel end and a bool — both `Unpin`, so this never has to
        // project.
        let this = self.get_mut();
        let Some(responses) = this.response_receiver.as_mut() else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(responses.poll_recv(cx)) else {
            this.response_receiver = None;
            return Poll::Ready(Some(Err(McpStreamError::Closed)));
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(McpStreamError::Frame(error))));
            }
        };
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => {
                payload
            }
            // The finish, which is the exchange ending as it should.
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                this.response_receiver = None;
                return Poll::Ready(None);
            }
            _ => {
                this.response_receiver = None;
                return Poll::Ready(Some(Err(McpStreamError::Misrouted)));
            }
        };
        Poll::Ready(Some(
            match channel_response::mcp::Frame::decode(payload) {
                Ok(channel_response::mcp::Frame::Head(head)) => {
                    if this.headed {
                        this.response_receiver = None;
                        Err(McpStreamError::SecondHead)
                    } else {
                        this.headed = true;
                        Ok(McpFrame::Head(head))
                    }
                }
                Ok(channel_response::mcp::Frame::Body(body)) => {
                    if this.headed {
                        // A refcounted view of exactly the body, which
                        // is what lets a piece of the answer be kept
                        // rather than copied. The slice came out of
                        // `bytes` a moment ago, so it is a subset of
                        // it.
                        Ok(McpFrame::Body(bytes.slice_ref(body)))
                    } else {
                        this.response_receiver = None;
                        Err(McpStreamError::HeadlessBody)
                    }
                }
                Err(error) => {
                    this.response_receiver = None;
                    Err(McpStreamError::Response(error))
                }
            },
        ))
    }
}

/// Whether the exchange is over.
///
/// Free to answer, because the terminal state is a field rather than
/// something to work out.
impl FusedStream for McpStream {
    fn is_terminated(&self) -> bool {
        self.response_receiver.is_none()
    }
}

/// An MCP exchange that stopped without ending.
///
/// None of these is the far server refusing. There is no error frame on
/// an MCP channel — the exchange is HTTP and a refusal is a status — so
/// every one of these is the machinery underneath breaking, and a
/// caller that sees one learns nothing about what the server would have
/// said.
///
/// It is flat, and beside [`McpError`](super::McpError) rather than
/// inside it: one is an exchange that never started, this is one that
/// started and then stopped without ending.
#[derive(Debug)]
pub enum McpStreamError {
    /// The connection ended mid-answer.
    ///
    /// The pieces stopped without a finish, so what arrived is a prefix
    /// of the answer — and for a JSON document, a prefix is not a
    /// smaller document.
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
    /// Neither a response nor the finish.
    Misrouted,
    /// The answer did not parse.
    Response(response::FrameError),
    /// A body arrived before the head.
    ///
    /// The head is always first, so this is a peer that disagrees about
    /// the protocol rather than an answer this end could make sense of
    /// — there is no status yet to say what the bytes are.
    HeadlessBody,
    /// A second head arrived.
    ///
    /// The head comes once and is never repeated. A second one would
    /// mean the status and headers this end already acted on have been
    /// replaced, which nothing in HTTP allows and nothing here can
    /// reconcile.
    SecondHead,
}

impl fmt::Display for McpStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            McpStreamError::Closed => f.write_str(
                "the connection ended in the middle of an mcp exchange",
            ),
            McpStreamError::Frame(error) => {
                write!(f, "mcp frame did not decode: {error}")
            }
            McpStreamError::Misrouted => f.write_str(
                "a frame arrived that does not belong on an mcp exchange",
            ),
            McpStreamError::Response(error) => {
                write!(f, "mcp frame did not parse: {error}")
            }
            McpStreamError::HeadlessBody => {
                f.write_str("an mcp body arrived before the head")
            }
            McpStreamError::SecondHead => {
                f.write_str("a second mcp head arrived")
            }
        }
    }
}

impl std::error::Error for McpStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            McpStreamError::Frame(error) => Some(error),
            McpStreamError::Response(error) => Some(error),
            McpStreamError::Closed
            | McpStreamError::Misrouted
            | McpStreamError::HeadlessBody
            | McpStreamError::SecondHead => None,
        }
    }
}
