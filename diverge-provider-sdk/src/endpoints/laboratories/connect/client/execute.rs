//! Joining a laboratory.

use std::fmt;

use bytes::Bytes;

use super::execute_handle::ExecuteHandle;
use super::execute_stream::ExecuteStream;
use super::{channel_request, request};
use crate::client::handle::{Handle, SendError};
use crate::encode::{Encode, Writer};

/// Attach to a container somebody else is running.
///
/// # Why this one hands back two things
///
/// Because a connection is two jobs at once and neither is the other's
/// subject. The scope streams the container's filesystem for as long as
/// the connection lasts, and a connector reaches into the container on
/// channels of its own — reading files, writing them, calling the MCP
/// server, moving a file to another container.
///
/// Every other `execute` in this crate has one of those. A
/// [`watch`](crate::endpoints::volumes::watch) only listens, so a
/// stream is the whole of it; a
/// [`plugin`](crate::endpoints::mcp_plugin::run) is almost silent, so a
/// handle is. A connection is both, and folding them into one type
/// would have meant a caller that stopped reading the filetree had
/// stopped being able to read a file.
///
/// So the [`ExecuteStream`] is what the provider says, and the
/// [`ExecuteHandle`] is how a connector says anything. They are
/// independent: dropping the stream costs a view of the filesystem, and
/// dropping the handle leaves the laboratory.
///
/// A tuple rather than a type holding both, because a type holding both
/// would exist only to be taken apart.
///
/// # It answers nothing
///
/// A connection is the one endpoint where a caller is purely a caller.
/// A provider asks a connector for exactly one thing — the content of a
/// file the connector said it wanted to write — and asks for nothing on
/// its own account: the image was somebody else's problem and so was
/// deciding who may attach.
///
/// Which is why this takes no proxies, where a
/// [`plugin`](crate::endpoints::mcp_plugin::run::client::execute) takes
/// three, and why there is no task spawned beside it.
///
/// # It fails in only one way
///
/// The request either serializes or it does not, and then it either
/// goes out or it does not. Everything a provider might object to is
/// objected to afterwards, in a frame — including the runner refusing
/// the authorization, which is the ordinary way a connection does not
/// happen. See
/// [`ExecuteStreamError`](super::ExecuteStreamError).
///
/// # The disconnect is built here, not later
///
/// [`ExecuteHandle`] sends it when dropped, and a destructor is a poor
/// place to be encoding anything — so it is encoded now, through
/// [`channel_request::Frame`] rather than written as the byte it
/// happens to be. What a disconnect looks like on the wire is that
/// module's to say, and this is a caller like any other.
pub async fn execute(
    handle: &Handle,
    request: &request::Frame,
) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;

    let mut disconnect = Vec::new();
    channel_request::Frame::Disconnect
        .encode(&mut Writer::new(&mut disconnect))
        .map_err(ExecuteError::Disconnect)?;

    let scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    Ok((
        ExecuteStream::new(scope.response_receiver),
        ExecuteHandle::new(
            handle.clone(),
            scope.scope,
            Bytes::from(disconnect),
            scope.request_receiver,
        ),
    ))
}

/// A connection that never opened.
///
/// One of two ways to fail before there is anything to fail at, and
/// neither of them is a refusal — a runner saying no arrives as a frame
/// on a scope that opened to carry it. See
/// [`ExecuteStreamError`](super::ExecuteStreamError), which is the
/// connection that opened and then stopped without closing.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    ///
    /// Nothing was written, so nothing is waiting to answer it. See
    /// [`SendError`] for the three reasons, only one of which is about
    /// this exchange rather than the whole connection.
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The disconnect would not serialize.
    ///
    /// Which cannot happen — a disconnect is one tag byte — and is
    /// reported rather than unwrapped because the encode it shares an
    /// impl with can fail. It is built here, before the request goes
    /// out, because the only place it is used is a destructor: too late
    /// to serialize, and with nobody to tell if it went wrong.
    Disconnect(serde_json::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "the request never went out: {error}")
            }
            ExecuteError::Request(error) => {
                write!(f, "connection request did not serialize: {error}")
            }
            ExecuteError::Disconnect(error) => {
                write!(f, "connection disconnect did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Disconnect(error) => Some(error),
        }
    }
}
