//! Opening the scope and reading it until the cancel, or the end.

use super::super::request;
use super::super::super::server::response;
use crate::daemon::client::{stream, Cancel};
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;

/// A agents list that did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The stream: one [`response::Frame`] per response, ended by the
/// daemon's finish.
pub type ExecuteStream = stream::ExecuteStream<response::Frame, response::FrameError>;

/// Open the scope and hand back its stream and its [`Cancel`], without
/// reading anything: the daemon's first frame is the stream's. The
/// stream ends at the daemon's finish, which a cancel provokes; a
/// client that wants the list once reads to
/// [`Listed`](response::Frame::Listed) and cancels.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Cancel), OpenError> {
    stream::execute_cancellable(handle, request, |_, payload| response::Frame::decode(payload)).await
}
