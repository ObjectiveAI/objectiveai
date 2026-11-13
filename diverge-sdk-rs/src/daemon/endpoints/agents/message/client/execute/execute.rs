//! Sending the message and reading its fate, in one call; or opening
//! the scope and reading what the agent's log takes of it.

use super::super::request;
use super::super::super::server::response;
use crate::daemon::client::{one_shot, stream, Cancel};
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;

/// What an agents message could not do: the machinery, never the
/// daemon's answer, which [`execute`] hands back whole.
pub type ExecuteError = one_shot::Error<serde_json::Error, response::FrameError>;

/// A watched message that did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The stream of a watched message: one [`response::Frame`] per
/// response, ended by the daemon's finish.
pub type ExecuteStream = stream::ExecuteStream<response::Frame, response::FrameError>;

/// Send a request that does not watch and hand back the daemon's one
/// answer, whenever its fate is known — delivered, forbidden, the
/// daemon's error, or the refusal the agent's image's document makes
/// of it: see [`response::Frame`], and
/// [`one_shot`](crate::daemon::client::one_shot) for the exchange. A
/// request that watches is [`watch`]'s: here only its first answer
/// would be read.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<response::Frame, ExecuteError> {
    one_shot::execute(handle, request).await
}

/// Open the scope and hand back its stream and its [`Cancel`], without
/// reading anything: the daemon's first frame is the stream's. For a
/// request that watches, the stream is the items of the agent's log
/// that are the request's, or the one answer that it has none; for one
/// that does not, the one answer. The stream ends at the daemon's
/// finish, which a cancel provokes.
pub async fn watch(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Cancel), OpenError> {
    stream::execute_cancellable(handle, request, |_, payload| response::Frame::decode(payload)).await
}
