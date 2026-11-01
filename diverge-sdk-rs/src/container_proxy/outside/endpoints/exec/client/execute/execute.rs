//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteHandle, ExecuteStream};
use crate::shared::containers::exec;
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Run `command` in the container: its output as a stream, and the
/// handle that stops it. Dropping the stream abandons it, which kills
/// the command too.
pub async fn execute(handle: &Handle, command: exec::request::Request) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError> {
    let mut payload = Vec::new();
    request::Frame(command)
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    Ok((ExecuteStream::new(scope.response_receiver), ExecuteHandle::new(handle.clone(), scope.scope)))
}
