//! Sending the message and waiting for its fate, or calling it off.

use super::super::request;
use super::super::super::server::response;
use crate::daemon::client::{pending, stream, Pending};
use crate::wire::client::handle::Handle;

/// A message that did not go out.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// Send the message and hand back what waits for its fate:
/// [`Pending::wait`] reads the one answer — delivered, forbidden, or
/// the daemon's error, see [`response::Frame`], or `None` for the
/// finish with no answer before it, a cancel in time — and
/// [`Pending::cancel`] opens the cancel channel, after which whether an
/// answer comes says which came first.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<Pending<response::Frame>, OpenError> {
    pending::execute(handle, request).await
}
