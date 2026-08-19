//! Why a response stream stopped without ending.

use std::fmt;

use crate::frame;

/// A response stream that stopped without ending.
///
/// None of these is the stream finishing. That is [`None`] from the
/// stream, and the difference is the whole reason this type exists: a
/// stream that ended told a caller the provider is done, and a stream
/// that broke told it nothing.
///
/// Every one of them is the last item the stream yields.
///
/// # Two layers, and they are worth telling apart
///
/// [`Closed`](Self::Closed) and [`Frame`](Self::Frame) are the
/// stream's: the connection went, or what came off it was not a frame.
/// [`Payload`](Self::Payload) is the endpoint's, and it carries
/// whatever that endpoint's decoder returns — which is where a
/// provider's own error frame ends up, since telling one of those from
/// an answer is a thing only the endpoint knows how to do.
///
/// Nesting rather than flattening keeps each endpoint's error to what
/// it actually owns. An endpoint that had to carry these two as well
/// would be repeating them once per endpoint, and repeating the prose
/// that explains them.
#[derive(Debug)]
pub enum ResponseStreamError<E> {
    /// The connection ended mid-stream.
    ///
    /// The frames stopped without a finish, so what the provider would
    /// have said next is unknown — not nothing.
    ///
    /// On a CHANNEL this covers one more case: a scope ending takes its
    /// channels with it, so a channel whose scope finished sees its
    /// receiver close without a
    /// [`ChannelResponseFinish`](crate::frame::server::ServerFrame::ChannelResponseFinish)
    /// ever arriving.
    Closed,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own
    /// [`Router`](super::router::Router), which decodes the same bytes
    /// before forwarding them and discards what will not parse. It is
    /// here because [`Scope`](super::scope::Scope) and
    /// [`Channel`](super::channel::Channel) are public and their
    /// receivers could be fed by something else.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on this stream.
    ///
    /// Neither the item nor the finish — a channel's response on a
    /// scope's receiver, an auth frame, anything. Unreachable through
    /// this crate's own [`Router`](super::router::Router), which
    /// matches on the frame's type and gives each arm exactly one
    /// destination, so a scope's receiver only ever sees types `2` and
    /// `3` and a channel's only `5` and `6`.
    ///
    /// It is reported rather than treated as the end because that would
    /// be the one confusion this protocol works hardest to prevent: a
    /// stream that BROKE arriving as a stream that ENDED.
    Misrouted,
    /// The endpoint could not make an item out of the payload.
    ///
    /// Which includes a provider that reported a failure rather than
    /// an answer, if that endpoint's decoder folds one in — and they
    /// generally do, because a provider's error and a payload that will
    /// not parse are both reasons there is no item, and a caller does
    /// the same thing about either.
    Payload(E),
}

impl<E: fmt::Display> fmt::Display for ResponseStreamError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseStreamError::Closed => {
                f.write_str("connection ended in the middle of a stream")
            }
            ResponseStreamError::Frame(error) => {
                write!(f, "stream frame did not decode: {error}")
            }
            ResponseStreamError::Misrouted => f.write_str(
                "a frame arrived that does not belong on this stream",
            ),
            ResponseStreamError::Payload(error) => write!(f, "{error}"),
        }
    }
}

/// [`Payload`](ResponseStreamError::Payload) is its own source, because
/// it is somebody else's error carried whole rather than a failure this
/// type had.
impl<E: std::error::Error + 'static> std::error::Error
    for ResponseStreamError<E>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ResponseStreamError::Frame(error) => Some(error),
            ResponseStreamError::Payload(error) => Some(error),
            ResponseStreamError::Closed | ResponseStreamError::Misrouted => {
                None
            }
        }
    }
}
