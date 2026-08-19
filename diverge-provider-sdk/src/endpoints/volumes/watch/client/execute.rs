//! Starting a watch.

use std::fmt;

use bytes::Bytes;

use super::channel_request;
use super::execute_stream::ExecuteStream;
use super::request;
use crate::client::handle::Handle;
use crate::encode::{Encode, Writer};

/// Start watching a volume.
///
/// # Why this one does not hand back an answer
///
/// Every other volume endpoint collapses into a single call: one
/// question, one answer, then the scope is over, so there is a value to
/// return. A watch does not end. It sends a snapshot and then the
/// changes to it for as long as the scope stays open, and something
/// that returned one value would have had to pick a frame and throw the
/// rest away.
///
/// So it hands back an [`ExecuteStream`], which is where everything
/// about reading one lives — what ends it, what a caller owes it, and
/// what dropping it does.
///
/// # It fails in only one way
///
/// The request either serializes or it does not. Everything after that
/// belongs to the watch rather than to the asking — a provider that
/// refuses is refusing the watch, and it says so in a frame like
/// everything else. See
/// [`ExecuteStreamError`](super::ExecuteStreamError).
///
/// # It keeps the responses and lets the rest of the scope go
///
/// A [`Scope`](crate::client::scope::Scope) carries a second receiver,
/// for channels a provider opens inside it. Nothing is supposed to open
/// one inside a watch — but nothing forbids it either, and that
/// receiver is bounded at `1` while the router AWAITS it. Holding one
/// unread would mean a provider that opened two channels blocked the
/// router forever, stalling every scope on the connection.
///
/// So it is dropped here and a stray channel request dead-letters,
/// which is the rule [`Scope`](crate::client::scope::Scope) states
/// about itself: drop what you are not going to read.
///
/// # The disconnect is built here, not later
///
/// [`ExecuteStream`] sends it when dropped, and a destructor is a poor
/// place to be encoding anything — so it is encoded now, through
/// [`channel_request::Frame`] rather than written as the byte it
/// happens to be. What a disconnect looks like on the wire is that
/// module's to say, and this is a caller like any other.
///
/// # Choosing the capacity
///
/// It belongs to the caller because only the caller knows what it is
/// watching. A directory nobody touches sends a snapshot and goes
/// quiet; one under a build sends thousands of frames a second.
///
/// The unit is whole frames, headers included, and a frame is as large
/// as whatever the provider chunked — the memory is its choice of
/// chunk, not this one's.
///
/// Depth buys tolerance for a reader that falls behind and costs memory
/// while it does. Too shallow loses nothing — the router waits rather
/// than dropping — but it waits for every scope on the connection, not
/// just this one. Zero is not allowed and panics inside
/// [`Handle::send_request`], after the request has been encoded and
/// before anything reaches the wire, which is
/// [`tokio`](tokio::sync::mpsc::channel)'s rule rather than this one.
///
/// The second capacity is `1` and not offered, because what it feeds is
/// dropped before this returns.
pub async fn execute(
    handle: &Handle,
    request: &request::Frame,
    capacity: usize,
) -> Result<ExecuteStream, ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload, capacity, 1).await;
    let number = scope.scope;
    let mut disconnect_request = Vec::new();
    channel_request::Frame
        .encode(&mut Writer::new(&mut disconnect_request))
        .unwrap_or_else(|error| match error {});
    Ok(ExecuteStream::new(
        scope.response_receiver,
        handle.clone(),
        number,
        Bytes::from(disconnect_request),
    ))
}

/// A watch that never started.
///
/// One way, because starting one is only serializing the request and
/// writing it. Everything a provider might object to is objected to
/// afterwards, in a frame — see
/// [`ExecuteStreamError`](super::ExecuteStreamError), which is the
/// watch that started and then stopped without ending.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request would not serialize.
    Request(postcard::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Request(error) => {
                write!(f, "watch request did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Request(error) => Some(error),
        }
    }
}
