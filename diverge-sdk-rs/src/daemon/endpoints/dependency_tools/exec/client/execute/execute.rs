//! Opening the scope, and its stream.

use super::super::super::server::response;
use super::super::request;
use crate::daemon::client::{Cancel, stream};
use crate::wire::client::handle::Handle;

/// Why the scope did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The daemon's answers, in order, until the finish.
pub type ExecuteStream = stream::ExecuteStream<response::Frame, response::FrameError>;

/// Run the command: its stream, and the cancel that kills it. Dropping
/// the stream abandons it, which kills the command too.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Cancel), OpenError> {
    stream::execute_cancellable(handle, request, response::decode).await
}
