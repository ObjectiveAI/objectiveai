//! Asking and reading the answer, in one call.

use super::super::request;
use super::super::super::server::response;
use crate::daemon::client::one_shot;
use crate::wire::client::handle::Handle;

/// What a postgres get could not do: the machinery, never the daemon's
/// answer, which [`execute`] hands back whole.
pub type ExecuteError = one_shot::Error<serde_json::Error, response::FrameError>;

/// Send the request and hand back the daemon's one answer, whichever it
/// is: see [`response::Frame`], and
/// [`one_shot`](crate::daemon::client::one_shot) for the exchange.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<response::Frame, ExecuteError> {
    one_shot::execute(handle, request).await
}
