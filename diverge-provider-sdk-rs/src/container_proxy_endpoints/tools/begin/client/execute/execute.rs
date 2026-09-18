//! Opening the scope, and hearing that it has begun.

use serde_json::Value;

use super::super::request;
use super::super::super::server::{self, response};
use super::{ExecuteError, ExecuteHandle, Finish};
use crate::client::handle::Handle;
use crate::container_proxy_endpoints::client::{Ask, Asks};
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::frame;
use crate::shared::containers::tools::Tool;

/// Begin the server's work on a tool container: open the scope
/// carrying `arguments`, and wait for the proxy to say it has begun.
///
/// Reads exactly one frame off the main stream before returning,
/// because nothing may be opened on the scope before its `Begun`: a
/// refusal is [`Refused`](ExecuteError::Refused), the container's
/// server's own words; a finish first is
/// [`Unanswered`](ExecuteError::Unanswered). The main stream then
/// carries nothing more on a tool container, and the [`Finish`] is
/// how its end is heard. What comes back is the handle, the asks the
/// proxy will open, the finish, and the tools the container declared.
pub async fn execute(
    handle: &Handle,
    arguments: Value,
) -> Result<(ExecuteHandle, Asks<Ask>, Finish, Vec<Tool>), ExecuteError> {
    let mut payload = Vec::new();
    request::Frame { arguments }
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
    let tools = match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Begun(tools) => tools,
        response::Frame::Error(error) => return Err(ExecuteError::Refused(error)),
    };

    Ok((
        ExecuteHandle::new(handle.clone(), scope.scope),
        Asks::new(scope.request_receiver, decode_ask),
        Finish::new(scope.response_receiver),
        tools,
    ))
}

/// This family's asks, decoded into the shared owned form.
fn decode_ask(payload: &[u8]) -> Option<Ask> {
    server::channel_request::Frame::decode(payload).ok().map(Ask::from)
}
