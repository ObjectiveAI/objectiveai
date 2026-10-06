//! Sending the files and hearing the answer.

use bytes::Bytes;
use futures_util::Stream;

use super::super::request;
use super::super::super::server::{channel_request, response};
use crate::daemon::client::upload;
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;
use crate::shared::error::Error;

/// What a tools upload could not do: the machinery, never the daemon's
/// answer, which [`execute`] hands back whole.
pub type UploadError = upload::UploadError<serde_json::Error, response::FrameError>;

/// Send the request, answer every content channel the daemon opens from
/// `source` — asked with the file's path as the request named it, or
/// `None` for a file upload — and hand back the daemon's one answer,
/// whichever it is: see [`response::Frame`], and
/// [`upload`](crate::daemon::client::upload) for the exchange.
pub async fn execute<S, F>(handle: &Handle, request: &request::Frame, source: F) -> Result<response::Frame, UploadError>
where
    S: Stream<Item = Result<Bytes, Error>> + Send + 'static,
    F: FnMut(Option<String>) -> Option<S>,
{
    upload::execute(handle, request, decode_ask, source).await
}

/// The content ask, read: which file.
fn decode_ask(payload: &[u8]) -> Result<Option<String>, serde_json::Error> {
    channel_request::Frame::decode(payload).map(|ask| ask.path)
}
