//! Opening the scope and reading it to the end.

use super::super::request;
use super::super::super::server::response;
use crate::daemon::client::stream;
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;

/// A agents templates list that did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The stream: one [`response::Frame`] per response, ended by the
/// daemon's finish.
pub type ExecuteStream = stream::ExecuteStream<response::Frame, response::FrameError>;

/// Open the scope and hand back its stream, without reading anything:
/// the daemon's first frame is the stream's. A finish with nothing
/// before it is an empty answer, not a failure.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteStream, OpenError> {
    stream::execute(handle, request, |_, payload| response::Frame::decode(payload)).await
}
