//! Asking how much the caller may still use.

use std::fmt;

use super::super::super::server::response;
use super::super::request;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Ask a provider how much the caller may still use: the four numbers
/// of [`response::Frame`].
///
/// # It takes no request
///
/// A capacity request carries nothing, so there is nothing to hand
/// over — as with [`info`](crate::provider::endpoints::info).
///
/// # It reads one frame and leaves
///
/// The answer is JSON, decoded into [`response::Frame`] with nothing
/// borrowed from the bytes it arrived in. This returns as soon as it
/// has one frame; the finish that follows is never read, and dropping
/// the [`Scope`](crate::wire::client::scope::Scope) tells the router
/// nobody is listening for the rest. There is no timeout here or
/// anywhere else in this protocol: a provider that never answers is
/// waited on until the connection dies, which is
/// [`ExecuteError::Closed`].
///
/// # It fails five ways and none of them is a refusal
///
/// A capacity response has no failure variant: a provider answers
/// every capacity request. So every one of these is the machinery
/// underneath breaking — or a provider that finished the scope
/// without answering, [`ExecuteError::Unanswered`], which the wire
/// reads as a request not served.
pub async fn execute(handle: &Handle) -> Result<response::Frame, ExecuteError> {
    let mut payload = Vec::new();
    request::Frame
        .encode(&mut Writer::new(&mut payload))
        .unwrap_or_else(|error| match error {});
    let mut scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    let bytes = scope
        .response_receiver
        .recv()
        .await
        .ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(ExecuteError::Frame)?;
    // A router puts only a response and its finish on a scope's
    // response stream, so the only other thing this can be is a finish
    // arriving first — a scope that ended without saying anything.
    let frame::server::ServerFrame::Response { payload, .. } = envelope else {
        return Err(ExecuteError::Unanswered);
    };
    response::Frame::decode(payload).map_err(ExecuteError::Response)
}

/// A capacity request that did not produce an answer.
///
/// All five are this end's view of something going wrong. None is the
/// provider refusing, because a capacity answer has no way to refuse
/// — see [`execute`].
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The connection ended before an answer arrived.
    Closed,
    /// The provider finished the scope without answering: the
    /// request was not served.
    Unanswered,
    /// What came back was not a frame. Unreachable through this
    /// crate's own [`Router`](crate::wire::client::router::Router),
    /// which decodes the same bytes before forwarding them.
    Frame(frame::FrameError),
    /// The answer did not parse as the capacity it is.
    Response(serde_json::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "the request never went out: {error}")
            }
            ExecuteError::Closed => f.write_str("the connection ended before the capacity arrived"),
            ExecuteError::Unanswered => f.write_str("the scope finished without the capacity"),
            ExecuteError::Frame(error) => {
                write!(f, "capacity answer did not decode: {error}")
            }
            ExecuteError::Response(error) => {
                write!(f, "capacity answer did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed | ExecuteError::Unanswered => None,
        }
    }
}
