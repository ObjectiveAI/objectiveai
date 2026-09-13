//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteStream};
use crate::client::handle::Handle;
use crate::encode::{Encode, Writer};
use crate::shared::containers::read;

/// Read the file at `path`, as components from the container's root.
///
/// Reads nothing before returning: the proxy's first frame is the
/// first piece, or the error, and both are the stream's. A zero-byte
/// file is one empty piece; a finish with nothing before it is the
/// proxy that could not serve the read, and the stream simply ends.
/// The scope's inbox is dropped, because the proxy opens no channel
/// on a read.
pub async fn execute(handle: &Handle, path: Vec<String>) -> Result<ExecuteStream, ExecuteError> {
    let mut payload = Vec::new();
    request::Frame(read::request::Request { path })
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    Ok(ExecuteStream::new(scope.response_receiver))
}
