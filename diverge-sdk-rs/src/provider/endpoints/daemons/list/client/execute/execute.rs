//! Opening the scope.

use super::super::request;
use super::{Cancel, ExecuteError, ExecuteStream};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Ask the provider which connected identities are announced, and
/// keep asking.
///
/// Reads nothing before returning: the provider's first frame is the
/// first identity added or the word that the listing is whole, and
/// both are the stream's. The stream ends at the provider's finish,
/// which follows the [`Cancel`] sent or the connection ending; a caller
/// that wants the listing once reads to
/// [`Listed`](crate::provider::endpoints::daemons::list::server::response::Frame::Listed)
/// and stops. The scope's inbox is dropped, because the provider
/// opens no channel on a listing; what the caller opens, the cancel,
/// needs only the connection and the scope's number.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Cancel), ExecuteError> {
    let mut payload = Vec::new();
    // Taken as an argument like every other listing's, though it has
    // no fields to carry. Its encode is `Infallible`.
    request.encode(&mut Writer::new(&mut payload)).unwrap_or_else(|error| match error {});
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let cancel = Cancel::new(handle.clone(), scope.scope);
    Ok((ExecuteStream::new(scope.response_receiver), cancel))
}
