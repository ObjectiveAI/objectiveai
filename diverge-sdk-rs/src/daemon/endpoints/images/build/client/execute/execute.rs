//! Opening the scope and reading what the build says.

use super::super::request;
use super::super::super::server::response;
use crate::daemon::client::stream;
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;

/// An images build that did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The stream of a build: one [`response::Frame`] per response, ended
/// by the daemon's finish.
pub type ExecuteStream = stream::ExecuteStream<response::Frame, response::FrameError>;

/// Ask the daemon to build the image, and hand back the stream of
/// what it answers, without reading anything: zero or more lines of
/// the build's output, then the one answer that ends it — see
/// [`response::Frame`] — then the daemon's finish. There is no cancel:
/// a build is not stopped from the client's side.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteStream, OpenError> {
    stream::execute(handle, request, |_, payload| response::Frame::decode(payload)).await
}
