//! Opening the scope.

use super::super::request;
use super::super::super::server::response;
use super::{ExecuteError, ExecuteHandle};
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Serve the directory at `path`, as components from the container's
/// root, and wait for the proxy to say it is serving.
///
/// One frame is read: `Serving`, and the [`ExecuteHandle`] comes
/// back holding the scope; an error is [`Refused`](ExecuteError::Refused),
/// the proxy's own words. Nothing else arrives on the scope until the
/// finish that answers the stop, which the handle reads. The scope's
/// inbox is dropped, because the proxy opens no channel on a serve; a
/// stray one dead-letters rather than growing forever.
pub async fn execute(handle: &Handle, path: Vec<String>) -> Result<ExecuteHandle, ExecuteError> {
    let mut payload = Vec::new();
    request::Frame { path }
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let mut scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let bytes = scope.response_receiver.recv().await.ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(ExecuteError::Frame)?;
    let payload = match envelope {
        frame::server::ServerFrame::Response { payload, .. } => payload,
        frame::server::ServerFrame::ResponseFinish { .. } => return Err(ExecuteError::Unanswered),
        _ => return Err(ExecuteError::Misrouted),
    };
    match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Serving => Ok(ExecuteHandle::new(handle.clone(), scope.scope, scope.response_receiver)),
        response::Frame::Error(error) => Err(ExecuteError::Refused(error)),
    }
}
