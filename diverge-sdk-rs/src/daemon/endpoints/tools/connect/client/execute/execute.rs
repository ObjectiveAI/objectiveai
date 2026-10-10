//! Opening the scope and reading its one answer.

use std::fmt;

use super::super::request;
use super::super::super::server::response;
use super::ExecuteHandle;
use crate::shared::error::Error;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Open a connect on `handle` and read the daemon's one answer: the
/// [`ExecuteHandle`] when the tool is connected, else why it is not.
/// The scope's request stream is dropped unread: the daemon opens no
/// channel of its own on a connect.
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
        response::Frame::Connected => {}
        response::Frame::NotFound => return Err(ExecuteError::NotFound),
        response::Frame::Forbidden => return Err(ExecuteError::Forbidden),
        response::Frame::Error(error) => return Err(ExecuteError::Daemon(error)),
    }
    Ok(ExecuteHandle::new(handle.clone(), scope.scope, scope.response_receiver))
}

/// A connect that did not connect.
///
/// Seven of these are this end's view of something going wrong. The
/// last three are the daemon saying so itself, and they are the only
/// ones that mean the exchange worked: no tool is the one named, the
/// account is not allowed, or the daemon could not serve the tool.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The connection ended before anything came back.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it: the daemon could
    /// not serve the request at all.
    Unanswered,
    /// A frame arrived that does not belong on the main stream.
    Misrouted,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// No tool of the caller's is the one named.
    NotFound,
    /// The account the request is served for holds no grant allowing
    /// it.
    Forbidden,
    /// The daemon could not serve the tool, and said so: a dependency
    /// tool, a connected tool, or a container that would not start.
    Daemon(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => write!(f, "tools connect request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the connect answered"),
            ExecuteError::Frame(error) => write!(f, "tools connect answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the connect finished without an answer"),
            ExecuteError::Misrouted => f.write_str("a frame arrived that does not belong on the main stream"),
            ExecuteError::Response(error) => write!(f, "tools connect answer did not parse: {error}"),
            ExecuteError::NotFound => f.write_str("no tool is the one named"),
            ExecuteError::Forbidden => f.write_str("the account is not allowed to connect to the tool"),
            ExecuteError::Daemon(_) => f.write_str("the daemon could not serve the tool"),
        }
    }
}

impl std::error::Error for ExecuteError {
    /// [`Daemon`](ExecuteError::Daemon) has no source: what it carries
    /// is not a Rust error and deliberately does not implement one —
    /// see [`shared::error::Error`](crate::shared::error::Error).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::Misrouted
            | ExecuteError::NotFound
            | ExecuteError::Forbidden
            | ExecuteError::Daemon(_) => None,
        }
    }
}
