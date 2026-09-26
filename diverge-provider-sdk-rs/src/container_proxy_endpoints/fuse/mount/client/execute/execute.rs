//! Opening the scope, and hearing that the mount is made.

use super::super::request;
use super::super::super::server::{self, response};
use super::{Ask, ExecuteError, ExecuteHandle};
use crate::client::handle::Handle;
use crate::container_proxy_endpoints::client::Asks;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::frame;
use crate::shared::containers::fuse::Kind;

/// Make one FUSE mount at `path`, of `kind`, and wait for the proxy
/// to say it is made.
///
/// Reads exactly one frame off the main stream before returning,
/// because a mount is complete before the server goes on to the next
/// step: `Ok` is the mount made and the scope open for its life; an
/// error is [`Refused`](ExecuteError::Refused), the proxy's own words
/// for why not; a finish first is
/// [`Unanswered`](ExecuteError::Unanswered). What comes back is the
/// handle and the asks the mount will make.
pub async fn execute(handle: &Handle, path: Vec<String>, kind: Kind) -> Result<(ExecuteHandle, Asks<Ask>), ExecuteError> {
    let mut payload = Vec::new();
    request::Frame { path, kind }
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
        response::Frame::Ok => {}
        response::Frame::Error(message) => return Err(ExecuteError::Refused(message.to_owned())),
        // A mount is not a mutation of anything a volume keeps; a proxy
        // that says so has misspoken, and the mount is not made.
        response::Frame::ReadOnly => return Err(ExecuteError::Refused("read-only".to_owned())),
    }

    Ok((
        ExecuteHandle::new(handle.clone(), scope.scope),
        Asks::new(scope.request_receiver, decode_ask),
    ))
}

/// The mount's asks, decoded into their owned form.
fn decode_ask(payload: &[u8]) -> Option<Ask> {
    server::channel_request::Frame::decode(payload).ok().map(Ask::from)
}
