//! Asking and reading the answer, in one call.

use std::fmt;

use super::super::request;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::images::stat::server::response;
use crate::wire::frame;
use crate::shared::error::Error;

/// Examine an image, and wait for the answer.
///
/// One question, one answer, then the scope is over — so there is a
/// value to return, and returning it is the whole of what a caller
/// wanted. A [`Stat`](response::Stat) is an ANSWER, and it is the
/// only one; everything else is an [`ExecuteError::Provider`] — an
/// image the caller has none by that name, or a provider that could
/// not look.
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
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<response::Stat, ExecuteError> {
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
        response::Frame::Stat(stat) => Ok(stat),
        response::Frame::Error(error) => Err(ExecuteError::Provider(error)),
    }
}

/// A stat that did not produce an answer.
///
/// All but the last are this end's view of something going wrong. The
/// last is the provider saying so itself, and it is the only one that
/// means the exchange worked.
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
    Response(response::FrameDecodeError),
    /// The provider could not do it, and said so. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => write!(f, "image stat request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the image stat answered"),
            ExecuteError::Frame(error) => write!(f, "image stat answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("image stat finished without an answer"),
            ExecuteError::Response(error) => write!(f, "image stat answer did not parse: {error}"),
            ExecuteError::Provider(_) => f.write_str("the provider could not complete the image stat"),
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
            ExecuteError::Closed | ExecuteError::Unanswered | ExecuteError::Provider(_) => None,
        }
    }
}
