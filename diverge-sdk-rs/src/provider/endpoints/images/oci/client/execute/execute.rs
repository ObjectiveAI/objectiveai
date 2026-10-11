//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteHandle};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Open a pull on `handle`, and hold the scope.
///
/// Reads nothing before returning: the provider answers nothing on
/// the main stream, and every answer comes on an ask's own channel.
/// The scope's inbox is dropped, because the provider opens no
/// channel on a pull. A scope the provider will not serve finishes
/// unasked, which [`wait`](ExecuteHandle::wait) reports and every
/// ask after it fails to open.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteHandle, ExecuteError> {
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
    Ok(ExecuteHandle::new(handle.clone(), scope.scope, scope.response_receiver))
}
