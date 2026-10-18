//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteStream};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Ask the provider to build the image, and read what the build says.
///
/// Reads nothing before returning: the provider's first frame is the
/// build's first line of output, or its result, and all of them are
/// the stream's. The stream ends at the provider's finish, which
/// follows the result; the scope's inbox is dropped, because the
/// provider opens no channel on a build, and a caller opens none —
/// a build cannot be stopped from the caller's side, short of the
/// connection ending.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteStream, ExecuteError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    Ok(ExecuteStream::new(scope.response_receiver))
}
