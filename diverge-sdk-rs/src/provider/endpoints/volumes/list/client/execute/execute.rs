//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteStream, Stop};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Ask the provider which volumes the caller has, and keep asking.
///
/// Reads nothing before returning: the provider's first frame is the
/// first volume added, the word that the listing is whole, or the
/// error, and all of them are the stream's. The stream ends at the
/// provider's finish, which follows the [`Stop`] sent, the connection
/// ending, or the provider's error; a caller that wants the listing
/// once reads to [`Listed`](crate::provider::endpoints::volumes::list::server::response::Frame::Listed)
/// and stops. The scope's inbox is dropped, because the provider
/// opens no channel on a listing; what the caller opens, the stop,
/// needs only the connection and the scope's number.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Stop), ExecuteError> {
    let mut payload = Vec::new();
    // Taken as an argument like every other endpoint's, though it has
    // no fields to carry, so that all of them read alike and a field
    // added later changes nothing here. Its encode is `Infallible`,
    // and an empty match on one is how you say there is no value to
    // handle.
    request
        .encode(&mut Writer::new(&mut payload))
        .unwrap_or_else(|error| match error {});
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let stop = Stop::new(handle.clone(), scope.scope);
    Ok((ExecuteStream::new(scope.response_receiver), stop))
}
