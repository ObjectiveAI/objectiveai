//! What a caller's registry answers with, piece by piece.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;

use super::channel::Channel;
use crate::decode::Decode;
use crate::frame;
use crate::shared::http::response;

/// The head of a registry answer, or a piece of its body.
///
/// What an [`OciStream`] yields. The same two things
/// [`http::response::Frame`](crate::shared::http::response::Frame)
/// carries on the wire, owning its body instead of borrowing it — a
/// frame decoded out of a message borrows that message, and a stream
/// item outlives the poll that produced it.
///
/// Owning is not copying. The body is a refcounted view of the frame it
/// arrived in.
///
/// # The order is the protocol's
///
/// A [`Head`](Self::Head) comes once, first, and never again; every
/// item after it is a [`Body`](Self::Body). An [`OciStream`] reports a
/// second head as an error rather than passing it on, so a reader may
/// take the first item as the head and the rest as body without
/// checking.
#[derive(Debug, Clone, PartialEq)]
pub enum OciFrame {
    /// The status and headers, once.
    ///
    /// Where the distribution protocol says what it is saying: `404`
    /// for a blob that is not there, `206` for a partial pull that
    /// resumed, `Content-Length` for how much body follows.
    Head(response::Head),
    /// A piece of the body.
    ///
    /// A manifest arrives as one; a layer arrives as many, and how it
    /// is divided is the caller's business and carries no meaning.
    Body(Bytes),
}

/// One answer from the caller's registry.
///
/// What [`ClientRegistry::request`](super::client_registry::ClientRegistry::request)
/// gives back. The request has gone out and the caller is already
/// answering into it.
///
/// # A provider does not read this
///
/// Not really. It relays: the bytes belong to a container runtime that
/// asked for them, and what a manifest says or which layers it names is
/// something the runtime works out. A provider that parsed one would be
/// keeping a second index beside the runtime's, and the two would
/// disagree the first time either changed.
///
/// What a provider DOES read is the head, and only far enough to hand
/// it back — a status and its headers, written onto the runtime's own
/// socket, so that `Range` resumes and `HEAD` probes work because
/// nothing here had to be taught about them.
///
/// # Zero or more items, then one ending
///
/// It yields items and then either ends or yields exactly one [`Err`]
/// and ends. Every error is terminal, which is what makes the type
/// explainable in one line and what [`FusedStream`] then reports
/// honestly.
///
/// [`None`] is the answer finishing. A stream that ends having yielded
/// nothing is a caller that opened the channel and closed it without
/// saying anything, which is not something an
/// [`OciProxy`](crate::client::oci_proxy::OciProxy) can do — it always
/// answers a head — so it means the caller is not one.
///
/// # There is no failure of its own
///
/// A registry refusal is a status. `404` for a blob the caller does not
/// have, `502` for a store it cannot reach — both arrive as a
/// [`Head`](OciFrame::Head) on a stream that is working perfectly. Every
/// variant of [`OciStreamError`] is this crate's plumbing breaking.
#[must_use = "an answer that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct OciStream {
    /// The channel this answer is arriving on.
    ///
    /// [`None`] once the stream has ended, which is half the terminal
    /// state; [`headed`](Self::headed) is the other half.
    ///
    /// # The whole channel, not its receiver
    ///
    /// A [`Channel`] tells the
    /// [`Session`](super::session::Session) it is over when it drops,
    /// so taking the receiver out and letting the rest go would say the
    /// channel had closed while its answer was still arriving.
    ///
    /// Which makes ending the stream and ending the channel one act:
    /// this is taken out when there is no more to read, and the notice
    /// goes at that moment rather than whenever a caller happens to
    /// drop the stream.
    channel: Option<Channel>,
    /// Whether the head has been seen.
    ///
    /// The protocol says a head comes once and never again, so this is
    /// what lets a second one be reported rather than passed on — and
    /// what lets a body arriving first be reported too, which is the
    /// same rule read from the other side.
    headed: bool,
}

impl OciStream {
    /// Take the channel's answers, from the request that asked for
    /// them.
    ///
    /// Not public. An answer exists because a request went out, so the
    /// only thing that can honestly make one of these is the thing that
    /// sent it.
    pub(super) fn new(channel: Channel) -> Self {
        OciStream {
            channel: Some(channel),
            headed: false,
        }
    }
}

/// The head, then the body, until there is no more.
impl Stream for OciStream {
    type Item = Result<OciFrame, OciStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // A channel end and a bool — both `Unpin`, so this never has to
        // project.
        let this = self.get_mut();
        let Some(channel) = this.channel.as_mut() else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(channel.response_receiver.poll_recv(cx))
        else {
            this.channel = None;
            return Poll::Ready(Some(Err(OciStreamError::Closed)));
        };
        let envelope = match frame::client::ClientFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => {
                this.channel = None;
                return Poll::Ready(Some(Err(OciStreamError::Frame(error))));
            }
        };
        let payload = match envelope {
            frame::client::ClientFrame::ChannelResponse { payload, .. } => {
                payload
            }
            // The finish, which is the answer ending as it should.
            frame::client::ClientFrame::ChannelResponseFinish { .. } => {
                this.channel = None;
                return Poll::Ready(None);
            }
            _ => {
                this.channel = None;
                return Poll::Ready(Some(Err(OciStreamError::Misrouted)));
            }
        };
        Poll::Ready(Some(match response::Frame::decode(payload) {
            Ok(response::Frame::Head(head)) => {
                if this.headed {
                    this.channel = None;
                    Err(OciStreamError::SecondHead)
                } else {
                    this.headed = true;
                    Ok(OciFrame::Head(head))
                }
            }
            Ok(response::Frame::Body(body)) => {
                if this.headed {
                    // A refcounted view of exactly the body. The slice
                    // came out of `bytes` a moment ago, so it is a
                    // subset of it.
                    Ok(OciFrame::Body(bytes.slice_ref(body)))
                } else {
                    this.channel = None;
                    Err(OciStreamError::HeadlessBody)
                }
            }
            Err(error) => {
                this.channel = None;
                Err(OciStreamError::Response(error))
            }
        }))
    }
}

/// Whether the answer is over.
impl FusedStream for OciStream {
    fn is_terminated(&self) -> bool {
        self.channel.is_none()
    }
}

/// A registry answer that stopped without ending.
///
/// None of these is the caller refusing. A refusal is a status, so
/// every one of these is the machinery underneath breaking — and a
/// provider that sees one has an image it cannot pull and a runtime
/// waiting on bytes that are not coming.
#[derive(Debug)]
pub enum OciStreamError {
    /// The connection ended mid-answer.
    ///
    /// What arrived is a prefix of the body, and for a layer a prefix
    /// is not a smaller layer.
    Closed,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own
    /// [`Session`](super::session::Session), which decodes the same
    /// bytes before forwarding them and discards what will not parse.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on a channel's answer.
    Misrouted,
    /// The answer did not parse.
    Response(response::FrameError),
    /// A body arrived before the head.
    ///
    /// The head is always first, so this is a caller that disagrees
    /// about the protocol rather than an answer that could be made
    /// sense of — there is no status yet to say what the bytes are.
    HeadlessBody,
    /// A second head arrived.
    ///
    /// The head comes once and is never repeated. A second one would
    /// mean the status and headers already relayed to the runtime have
    /// been replaced, which nothing in HTTP allows.
    SecondHead,
}

impl fmt::Display for OciStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OciStreamError::Closed => f.write_str(
                "the connection ended in the middle of a registry answer",
            ),
            OciStreamError::Frame(error) => {
                write!(f, "registry frame did not decode: {error}")
            }
            OciStreamError::Misrouted => f.write_str(
                "a frame arrived that does not belong on a registry answer",
            ),
            OciStreamError::Response(error) => {
                write!(f, "registry frame did not parse: {error}")
            }
            OciStreamError::HeadlessBody => {
                f.write_str("a registry body arrived before the head")
            }
            OciStreamError::SecondHead => {
                f.write_str("a second registry head arrived")
            }
        }
    }
}

impl std::error::Error for OciStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OciStreamError::Frame(error) => Some(error),
            OciStreamError::Response(error) => Some(error),
            OciStreamError::Closed
            | OciStreamError::Misrouted
            | OciStreamError::HeadlessBody
            | OciStreamError::SecondHead => None,
        }
    }
}
