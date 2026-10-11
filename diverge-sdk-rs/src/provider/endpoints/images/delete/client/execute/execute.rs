//! Asking and reading the answer, in one call.

use std::fmt;

use super::super::request;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::images::delete::server::response;
use crate::wire::frame;
use crate::shared::error::Error;

/// Destroy an image, and wait for the answer.
///
/// One question, one answer, then the scope is over. There is one
/// answer and it carries nothing: the image is gone. An image a
/// running container of the caller runs is [`ExecuteError::InUse`] —
/// the protocol's defined refusal, not a failure, and the one a
/// caller acts on by stopping the container and asking again.
/// Everything else is an [`ExecuteError::Provider`] — an image the
/// caller has none by that name, or a provider that could not do it.
///
/// # It reads one frame and leaves
///
/// The scope's queues are unbounded and this takes one thing off one
/// of them. The request stream is dropped unread: nothing is supposed
/// to arrive on it. There is no timeout here or anywhere else in this
/// protocol: a provider that never answers is waited on until the
/// connection dies, at which point this returns
/// [`ExecuteError::Closed`]. The finish that follows the answer is
/// never read; dropping the scope is what tells the router nobody is
/// listening for the rest.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(), ExecuteError> {
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
        response::Frame::Deleted => Ok(()),
        response::Frame::InUse => Err(ExecuteError::InUse),
        response::Frame::Error(error) => Err(ExecuteError::Provider(error)),
    }
}

/// A deletion that did not produce an answer.
///
/// All but the last two are this end's view of something going wrong.
/// The last two are the provider saying so itself, and they are the
/// only ones that mean the exchange worked.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The request would not serialize.
    Request(postcard::Error),
    /// The connection ended before anything came back, which is the
    /// only way waiting stops early.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it.
    Unanswered,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// A running container of the caller runs the image, and the
    /// provider does not delete an image in use. Nothing was changed.
    InUse,
    /// The provider could not do it, and said so. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => write!(f, "image deletion request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the image deletion answered"),
            ExecuteError::Frame(error) => write!(f, "image deletion answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("image deletion finished without an answer"),
            ExecuteError::Response(error) => write!(f, "image deletion answer did not parse: {error}"),
            ExecuteError::InUse => f.write_str("a running container runs the image, and it was not deleted"),
            ExecuteError::Provider(_) => f.write_str("the provider could not complete the image deletion"),
        }
    }
}

impl std::error::Error for ExecuteError {
    /// [`Provider`](ExecuteError::Provider) has no source, because what
    /// it carries is not a Rust error and deliberately does not
    /// implement one; a caller that wants what is inside it matches
    /// the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed | ExecuteError::Unanswered | ExecuteError::InUse | ExecuteError::Provider(_) => None,
        }
    }
}
