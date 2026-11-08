//! Asking and reading the answer, in one call.

use std::fmt;

use super::super::request;
use crate::provider::endpoints::images::hash::server::response;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Ask for the proof over an image the provider can supply, and wait
/// for the answer.
///
/// One question, one answer, then the scope is over. The answer is
/// returned as the frame it is — the proof, the word that the provider
/// cannot supply the image, the word that it has neither a Store nor
/// any registry, or the provider's error — since a relay hands each
/// on as it came.
/// Everything that is not an answer is an [`ExecuteError`].
///
/// # It reads one frame and leaves
///
/// The scope's queues are unbounded and this takes one thing off one
/// of them. There is no timeout here or anywhere else in this
/// protocol: a provider that never answers is waited on until the
/// connection dies, at which point this returns
/// [`ExecuteError::Closed`]. The finish that follows the answer is
/// never read; dropping the scope is what tells the router nobody is
/// listening for the rest.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<response::Frame, ExecuteError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
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

/// A hash that did not produce an answer: this end's view of
/// something going wrong. The provider's own refusals are answers,
/// and come back as the frame.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The connection ended before anything came back, which is the
    /// only way waiting stops early.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it.
    Unanswered,
    /// The response frame did not parse.
    Response(response::FrameError),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => write!(f, "image hash request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the image hash answered"),
            ExecuteError::Frame(error) => write!(f, "image hash answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("image hash finished without an answer"),
            ExecuteError::Response(error) => write!(f, "image hash answer did not parse: {error}"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed | ExecuteError::Unanswered => None,
        }
    }
}
