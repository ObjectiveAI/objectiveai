//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteHandle, ExecuteStream};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Watch the container's tree, leaving `ignore` out.
///
/// Reads nothing before returning: the proxy's first frame is the
/// snapshot, or the error, and both are the stream's. The scope's
/// inbox is dropped, because the proxy opens no channel on a tree; a
/// stray one dead-letters rather than growing forever.
pub async fn execute(handle: &Handle, ignore: Vec<Vec<String>>) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError> {
    let mut payload = Vec::new();
    request::Frame { ignore }
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    Ok((
        ExecuteStream::new(scope.response_receiver),
        ExecuteHandle::new(handle.clone(), scope.scope),
    ))
}
