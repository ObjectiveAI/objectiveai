//! Asking for an image's document and reading the answer, in one call.

use std::fmt;

use super::super::request;
use crate::provider::endpoints::containers::info::server::response;
use crate::shared::containers::info::Document;
use crate::shared::error::Error;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Ask for an image's `/DIVERGE.md`, and wait for the answer: the
/// document, or `None` when the image ships none.
///
/// One question, one answer, then the scope is over, so there is a
/// value to return: the request is written, the one response frame is
/// read and taken out of its envelope, and the finish that follows it
/// is never read. Nothing times the provider out: a provider that
/// never answers is waited on until the connection ends, which is
/// [`ExecuteError::Closed`].
///
/// The two answers are `Some` and `None`. A provider that could find
/// no copy of the image is [`ExecuteError::NoSource`], and one that
/// failed — a file that is not a document among the ways — is
/// [`ExecuteError::Provider`].
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<Option<Document>, ExecuteError> {
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
    match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Info(document) => Ok(Some(document)),
        response::Frame::Missing => Ok(None),
        response::Frame::NoSource => Err(ExecuteError::NoSource),
        response::Frame::Error(error) => Err(ExecuteError::Provider(error)),
    }
}

/// An image info request that did not produce an answer.
///
/// Six of these are this end's view of something going wrong. The
/// last two are the provider speaking.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The connection ended before anything came back.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it.
    Unanswered,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// The provider could find the image in no store and no registry
    /// it may ask, and said so: nothing was read.
    NoSource,
    /// The provider could not answer, and said so — the file is there
    /// and is not a document, or the image could not be read.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => write!(f, "image info request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the image info answered"),
            ExecuteError::Frame(error) => write!(f, "image info answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("image info finished without an answer"),
            ExecuteError::Response(error) => write!(f, "image info answer did not parse: {error}"),
            ExecuteError::NoSource => f.write_str("the provider found no copy of the image"),
            ExecuteError::Provider(_) => f.write_str("the provider could not read the image's info"),
        }
    }
}

impl std::error::Error for ExecuteError {
    /// [`Provider`](ExecuteError::Provider) has no source, because what
    /// it carries is not a Rust error: see
    /// [`shared::error::Error`](crate::shared::error::Error).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed | ExecuteError::Unanswered | ExecuteError::NoSource | ExecuteError::Provider(_) => None,
        }
    }
}
