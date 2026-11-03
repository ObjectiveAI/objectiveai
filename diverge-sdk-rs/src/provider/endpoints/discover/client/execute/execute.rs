//! Asking which connected identities are discoverable.

use std::fmt;

use super::super::super::server::response;
use super::super::request;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Ask a provider which connected identities are discoverable: every
/// identity connected to it whose discover flag was `1`.
///
/// It takes no request, as [`info`](crate::provider::endpoints::info)'s
/// takes none: the request carries nothing beyond its tag. It reads one
/// frame — the answer — and leaves; the finish that follows is never
/// read, and dropping the
/// [`Scope`](crate::wire::client::scope::Scope) tells the router nobody
/// is listening for it. There is no timeout: a provider that never
/// answers is waited on until the connection dies.
///
/// Every failure is the machinery underneath breaking, since the
/// answer has no failure variant: a provider always knows what is
/// connected to it.
pub async fn execute(handle: &Handle) -> Result<response::Frame, ExecuteError> {
    let mut payload = Vec::new();
    request::Frame.encode(&mut Writer::new(&mut payload)).unwrap_or_else(|error| match error {});
    let mut scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let bytes = scope.response_receiver.recv().await.ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(ExecuteError::Frame)?;
    // A router puts only a response and its finish on a scope's
    // response stream, so the only other thing this can be is a finish
    // arriving first — a scope that ended without saying anything.
    let frame::server::ServerFrame::Response { payload, .. } = envelope else {
        return Err(ExecuteError::Unanswered);
    };
    response::Frame::decode(payload).map_err(ExecuteError::Response)
}

/// A discover request that did not produce an answer. None is the
/// provider refusing, because the answer has no way to refuse.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    Send(SendError),
    /// The connection ended before an answer arrived.
    Closed,
    /// The provider finished the scope without answering.
    Unanswered,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The answer did not parse as the listing it is.
    Response(serde_json::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Closed => f.write_str("the connection ended before the discoverable identities arrived"),
            ExecuteError::Unanswered => f.write_str("the scope finished without the discoverable identities"),
            ExecuteError::Frame(error) => write!(f, "discover answer did not decode: {error}"),
            ExecuteError::Response(error) => write!(f, "discover answer did not parse: {error}"),
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
