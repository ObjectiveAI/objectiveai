//! Opening the scope.

use super::super::request;
use super::{Cancel, ExecuteError, ExecuteStream};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Ask the provider to list the caller with the request's
/// announcement, and keep it listed.
///
/// Reads nothing before returning: the provider sends nothing while
/// the announce stands, and its one possible frame, the refusal, is
/// the stream's. The stream ends at the provider's finish, which
/// follows the [`Cancel`] sent or the connection ending; the caller is
/// listed from the provider reading the request until then.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Cancel), ExecuteError> {
    let mut payload = Vec::new();
    // Its encode is `Infallible`: a tag, and the announcement's bytes.
    request.encode(&mut Writer::new(&mut payload)).unwrap_or_else(|error| match error {});
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let cancel = Cancel::new(handle.clone(), scope.scope);
    Ok((ExecuteStream::new(scope.response_receiver), cancel))
}
