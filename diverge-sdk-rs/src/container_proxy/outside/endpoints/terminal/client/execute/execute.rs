//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteHandle, ExecuteStream};
use crate::shared::containers::terminal;
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Open a terminal at `size`: its output as a stream, and the handle
/// that writes its stdin, resizes it and stops it. Dropping the stream
/// abandons it, which kills the shell too.
pub async fn execute(handle: &Handle, size: terminal::open::Open) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError> {
    let mut payload = Vec::new();
    request::Frame(size)
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    Ok((ExecuteStream::new(scope.response_receiver), ExecuteHandle::new(handle.clone(), scope.scope)))
}
