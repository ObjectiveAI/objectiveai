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
use crate::shared::oci;

/// One answer from the caller's registry.
///
/// What [`ClientRegistry::request`](super::client_registry::ClientRegistry::request)
/// gives back. The request has gone out and the caller is already
/// answering into it.
///
/// The item is [`Bytes`] and nothing else — a refcounted view of the
/// frame it arrived in, never a copy. It is the mirror of what an
/// `OciProxy` produces, and the
/// same bytes.
///
/// # A provider does not read this
///
/// Not at all, which is stronger than it used to be. There was a head
/// here once, and a provider read it far enough to hand it back — which
/// meant reading a status, and then deciding what to do about the
/// headers that describe the MESSAGE rather than the answer. Every such
/// decision turned out to be a bug: a `Content-Length` copied onto a
/// body that had been re-encoded, a `Connection` forwarded past the hop
/// it belonged to.
///
/// So it writes them onto the runtime's own socket in the order they
/// arrive, and that is the whole of it. `Range` resumes and `HEAD`
/// probes work not because anything here was taught about them, but
/// because nothing here can get them wrong — the registry's answer
/// reaches the runtime as the registry wrote it.
///
/// A pull is bytes end to end. What a manifest says, which layers it
/// names, whether a `206` picked up where the last attempt stopped: all
/// of that is between the runtime and the registry, and a provider that
/// parsed any of it would be keeping a second index beside the
/// runtime's for the two to disagree over.
///
/// # Zero or more pieces, then one ending
///
/// It yields items and then either ends or yields exactly one [`Err`]
/// and ends. Every error is terminal, which is what makes the type
/// explainable in one line and what [`FusedStream`] then reports
/// honestly.
///
/// [`None`] is the answer finishing. Nothing in the pieces says the
/// answer is over, because the channel's finish already does — a
/// runtime told a `Content-Length` knows when it has the whole body,
/// and one reading a chunked answer knows from the terminator. Both of
/// those are inside the bytes, where they belong.
///
/// A stream that ends having yielded nothing is a caller that opened
/// the channel and closed it without saying anything. That is not
/// something to report here: it reaches the runtime as an empty answer,
/// which HTTP has its own opinion about.
///
/// # There is no failure of its own
///
/// A registry refusal is a status. `404` for a blob the caller does not
/// have, `502` for a store it cannot reach — both arrive as ordinary
/// bytes on a stream that is working perfectly. Every variant of
/// [`OciStreamError`] is this crate's plumbing breaking.
///
/// Which is why this is fallible where an
/// `OciProxy`'s stream is not.
/// That end produces bytes it already has; this end reads a wire, and a
/// wire can stop.
#[must_use = "an answer that is not polled grows a queue nobody reads"]
#[derive(Debug)]
pub struct OciStream {
    /// The channel this answer is arriving on.
    ///
    /// [`None`] once the stream has ended, which is the whole of the
    /// terminal state.
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
        }
    }
}

/// The answer, piece by piece, until there is no more.
impl Stream for OciStream {
    type Item = Result<Bytes, OciStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // One channel end, and it is `Unpin`, so this never has to
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
        // Through the shared type rather than straight off the payload.
        // It is the same bytes either way, and this crate is what says
        // so — reading an answer as the thing this protocol defines an
        // answer to be costs nothing and keeps the two ends named by
        // one type.
        let piece = oci::response::Frame::decode(payload)
            .unwrap_or_else(|error| match error {});
        // A refcounted view of exactly the piece. The slice came out of
        // `bytes` a moment ago, so it is a subset of it.
        Poll::Ready(Some(Ok(bytes.slice_ref(piece.0))))
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
///
/// # Three, where there were six
///
/// The other three were about a head: an answer that did not parse, a
/// body that arrived before the head, a second head after the first.
/// None of them can happen to bytes. There is nothing to parse, no
/// order for the pieces to be in, and nothing that comes once.
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
        }
    }
}

impl std::error::Error for OciStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OciStreamError::Frame(error) => Some(error),
            OciStreamError::Closed | OciStreamError::Misrouted => None,
        }
    }
}
