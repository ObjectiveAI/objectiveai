//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteHandle, ExecuteStream};
use crate::shared::containers::frontend;
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Open a frontend request with `head`: the answer as a stream, and the
/// handle that writes its body and cancels it. Dropping the stream
/// abandons it, which drops the request too.
pub async fn execute(handle: &Handle, head: frontend::request::Head) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError> {
    let mut payload = Vec::new();
    request::Frame(head)
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    Ok((ExecuteStream::new(scope.response_receiver), ExecuteHandle::new(handle.clone(), scope.scope)))
}
