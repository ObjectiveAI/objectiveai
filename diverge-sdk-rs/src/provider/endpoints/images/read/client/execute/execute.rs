//! Opening the scope, and reading whether it is held.

use super::super::request;
use super::super::super::server::response;
use super::{ExecuteError, ExecuteHandle};
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Open a read scope of the one image `request` names on `handle`,
/// read the provider's answer, and hold the scope where it is held.
///
/// The first frame on the main stream answers the request:
/// [`Held`](response::Frame::Held) is the [`ExecuteHandle`], every
/// ask on it answered on its own channel;
/// [`NotFound`](response::Frame::NotFound),
/// [`Unsupported`](response::Frame::Unsupported) and
/// [`Error`](response::Frame::Error) are [`ExecuteError::NotFound`],
/// [`ExecuteError::Unsupported`] and [`ExecuteError::Provider`], the
/// scope finishing behind them with no ask answered. A finish before
/// any answer is [`ExecuteError::Unanswered`].
///
/// The scope's inbox is dropped, because the provider opens no
/// channel on a read scope. There is no timeout here or anywhere else
/// in this protocol: a provider that never answers is waited on until
/// the connection dies, at which point this returns
/// [`ExecuteError::Closed`].
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteHandle, ExecuteError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
    let mut scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let bytes = scope.response_receiver.recv().await.ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(ExecuteError::Frame)?;
    // A router puts only a response and its finish on a scope's
    // response stream, so the only other thing this can be is a finish
    // arriving first — a scope that ended without saying anything.
    let frame::server::ServerFrame::Response { payload, .. } = envelope else {
        return Err(ExecuteError::Unanswered);
    };
    match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Held => Ok(ExecuteHandle::new(handle.clone(), scope.scope, scope.response_receiver)),
        response::Frame::NotFound => Err(ExecuteError::NotFound),
        response::Frame::Unsupported => Err(ExecuteError::Unsupported),
        response::Frame::Error(error) => Err(ExecuteError::Provider(error)),
    }
}
