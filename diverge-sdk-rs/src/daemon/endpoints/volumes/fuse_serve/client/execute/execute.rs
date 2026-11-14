//! Opening the scope.

use super::super::request;
use super::super::super::server::response;
use super::{ExecuteError, ExecuteHandle};
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Have the daemon serve the volume the request names, and wait for it
/// to say it is serving.
///
/// One frame is read: `Serving`, and the [`ExecuteHandle`] comes back
/// holding the scope; every other answer is its own [`ExecuteError`].
/// Nothing else arrives on the scope until its finish, which the
/// handle reads.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteHandle, ExecuteError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
    let mut scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let bytes = scope.response_receiver.recv().await.ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(ExecuteError::Frame)?;
    let payload = match envelope {
        frame::server::ServerFrame::Response { payload, .. } => payload,
        frame::server::ServerFrame::ResponseFinish { .. } => return Err(ExecuteError::Unanswered),
        _ => return Err(ExecuteError::Misrouted),
    };
    match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Serving => Ok(ExecuteHandle::new(handle.clone(), scope)),
        response::Frame::Mounted => Err(ExecuteError::Mounted),
        response::Frame::FuseServed => Err(ExecuteError::FuseServed),
        response::Frame::Held => Err(ExecuteError::Held),
        response::Frame::VolumeMode(refused) => Err(ExecuteError::VolumeMode(refused)),
        response::Frame::Unsupported => Err(ExecuteError::Unsupported),
        response::Frame::Forbidden => Err(ExecuteError::Forbidden),
        response::Frame::NotFound => Err(ExecuteError::NotFound),
        response::Frame::Error(error) => Err(ExecuteError::Daemon(error)),
    }
}
