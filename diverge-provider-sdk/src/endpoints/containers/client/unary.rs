//! Reading a client-opened channel that answers once.

use std::fmt;

use crate::client::handle::{Handle, SendError};
use crate::frame;

use super::Answered;

/// Open a channel with `payload` and take its one answer.
///
/// One frame, then the finish, is the shape of every unary exchange:
/// a write's confirmation, the agent's schema, a message's fate, a
/// tool's result. This reads the frame and leaves; the finish that
/// follows is never read, and dropping the channel's receiver is what
/// tells the router nobody is listening for it. A finish arriving
/// FIRST is a provider that could not serve the exchange at all —
/// [`Unanswered`](UnaryError::Unanswered), the wire's standing
/// meaning for it. There is no timeout: a provider that never answers
/// is waited on until the connection dies.
pub async fn unary<A: Answered>(
    handle: &Handle,
    scope: u32,
    payload: &[u8],
) -> Result<A::Item, UnaryError<A>> {
    let mut channel = handle
        .send_channel_request(scope, payload)
        .await
        .map_err(UnaryError::Send)?;
    let bytes = channel
        .response_receiver
        .recv()
        .await
        .ok_or(UnaryError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(UnaryError::Frame)?;
    let payload = match envelope {
        frame::server::ServerFrame::ChannelResponse { payload, .. } => payload,
        frame::server::ServerFrame::ChannelResponseFinish { .. } => {
            return Err(UnaryError::Unanswered);
        }
        _ => return Err(UnaryError::Misrouted),
    };
    match A::decode(bytes.slice_ref(payload)).map_err(UnaryError::Response)? {
        Ok(item) => Ok(item),
        Err(refusal) => Err(UnaryError::Refused(refusal)),
    }
}

/// A unary exchange that did not produce its answer.
#[derive(Debug)]
pub enum UnaryError<A: Answered> {
    /// The channel request would not serialize.
    Request(serde_json::Error),
    /// The channel request never went out.
    Send(SendError),
    /// The connection ended before anything came back.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The channel finished without an answer in it: the provider
    /// could not serve the exchange, and said nothing else.
    Unanswered,
    /// A frame arrived that does not belong on a channel's stream.
    Misrouted,
    /// The answer did not parse.
    Response(A::Error),
    /// The provider refused, in the wire's own words.
    Refused(A::Refusal),
}

impl<A: Answered> fmt::Display for UnaryError<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryError::Request(error) => write!(f, "channel request did not serialize: {error}"),
            UnaryError::Send(error) => write!(f, "the channel request never went out: {error}"),
            UnaryError::Closed => f.write_str("connection ended before the channel answered"),
            UnaryError::Frame(error) => write!(f, "channel answer did not decode: {error}"),
            UnaryError::Unanswered => f.write_str("the channel finished without an answer"),
            UnaryError::Misrouted => {
                f.write_str("a frame arrived that does not belong on a channel")
            }
            UnaryError::Response(error) => write!(f, "channel answer did not parse: {error}"),
            UnaryError::Refused(_) => f.write_str("the provider refused"),
        }
    }
}

impl<A: Answered> std::error::Error for UnaryError<A> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UnaryError::Request(error) => Some(error),
            UnaryError::Send(error) => Some(error),
            UnaryError::Frame(error) => Some(error),
            UnaryError::Response(error) => Some(error),
            UnaryError::Closed
            | UnaryError::Unanswered
            | UnaryError::Misrouted
            | UnaryError::Refused(_) => None,
        }
    }
}
