//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteStream};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Read the file the request names, out of the volume it names.
///
/// Reads nothing before returning: the provider's first frame is the
/// first piece, or the error, and both are the stream's. A zero-byte
/// file is one empty piece; a finish with nothing before it is the
/// provider that could not serve the read, and the stream simply
/// ends. The scope's inbox is dropped, because the provider opens no
/// channel on a read.
pub async fn execute(
    handle: &Handle,
    request: &request::Frame,
) -> Result<ExecuteStream, ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    Ok(ExecuteStream::new(scope.response_receiver))
}
