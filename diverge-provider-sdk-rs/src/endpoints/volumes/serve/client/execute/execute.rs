//! Opening the scope.

use super::super::request;
use super::super::super::server::response;
use super::{ExecuteError, ExecuteHandle};
use crate::client::handle::Handle;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::frame;

/// Serve the volume the request names, and wait for the provider to
/// say it is serving.
///
/// One frame is read: `Serving`, and the [`ExecuteHandle`] comes
/// back holding the scope; an error is [`Refused`](ExecuteError::Refused),
/// the provider's own words. Nothing else arrives on the scope until
/// the finish that answers the stop, which the handle reads.
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
        response::Frame::Error(error) => Err(ExecuteError::Refused(error)),
    }
}
