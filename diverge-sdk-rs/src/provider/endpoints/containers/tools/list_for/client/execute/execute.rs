//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteStream, Stop};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Ask the provider which tool containers the identity the request
/// names runs, and keep asking.
///
/// Reads nothing before returning: the provider's first frame is the
/// first container added, the word that the listing is whole, or the
/// error, and all of them are the stream's. The stream ends at the
/// provider's finish, which follows the [`Stop`] sent, the
/// connection ending, or the provider's error; a lister that wants
/// the listing once reads to [`Listed`](crate::provider::endpoints::containers::tools::list_for::server::response::Frame::Listed)
/// and stops. The scope's inbox is dropped, because the provider
/// opens no channel on a listing; what the lister opens, the stop,
/// needs only the connection and the scope's number.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Stop), ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let stop = Stop::new(handle.clone(), scope.scope);
    Ok((ExecuteStream::new(scope.response_receiver), stop))
}
