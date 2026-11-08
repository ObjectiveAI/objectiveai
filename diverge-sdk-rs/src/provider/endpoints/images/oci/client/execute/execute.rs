//! Opening the scope.

use super::super::request;
use super::{ExecuteError, ExecuteHandle};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Open a pull of the one image `request` names on `handle`, and hold
/// the scope.
///
/// Reads nothing before returning: a provider that serves the pull
/// answers nothing on the main stream, and every answer comes on an
/// ask's own channel. The scope's inbox is dropped, because the
/// provider opens no channel on a pull. A scope the provider will not
/// serve finishes unasked — after the one frame saying it cannot
/// supply the image, or has neither a Store nor any registry, where
/// that is why — which [`wait`](ExecuteHandle::wait) reports and every
/// ask after it fails to open.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteHandle, ExecuteError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    Ok(ExecuteHandle::new(handle.clone(), scope.scope, scope.response_receiver))
}
