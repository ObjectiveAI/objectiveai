//! Starting a watch, and reading it for as long as it lasts.

use std::fmt;

use bytes::Bytes;

use super::request;
use crate::client::handle::Handle;
use crate::client::scope_response_stream::ScopeResponseStream;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::watch::server::response;
use crate::shared::error::Error;
use crate::shared::filetree;

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
/// So it hands back a
/// [`ScopeResponseStream`](crate::client::scope_response_stream::ScopeResponseStream),
/// which is where everything about reading one lives — what ends it,
/// what a caller owes it, and what dropping it does not do.
///
/// # It fails in only one way
///
/// The request either serializes or it does not. Everything after that
/// belongs to the watch rather than to the asking — a provider that
/// refuses is refusing the watch, and it says so in a frame like
/// everything else. See [`WatchError`].
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
) -> Result<ScopeResponseStream<filetree::response::Frame, WatchError>, ExecuteError>
{
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload, capacity, 1).await;
    Ok(ScopeResponseStream::new(scope.response_receiver, decode))
}

/// One payload, as one change on the tree.
///
/// The whole of what is watch's about a watch stream. Everything else —
/// the receiver, the envelope, the ending, staying ended — belongs to
/// [`ScopeResponseStream`](crate::client::scope_response_stream::ScopeResponseStream).
///
/// The item is [`filetree::response::Frame`], the SHARED one. Taking
/// this endpoint's own envelope off it is the point: a caller folding a
/// tree wants the change, not the news that a change is what this is.
fn decode(
    payload: Bytes,
) -> Result<filetree::response::Frame, WatchError> {
    match response::Frame::decode(&payload) {
        Ok(response::Frame::Filetree(frame)) => Ok(frame),
        Ok(response::Frame::Error(error)) => Err(WatchError::Provider(error)),
        Err(error) => Err(WatchError::Response(error)),
    }
}

/// A watch that never started.
///
/// One way, because starting one is only serializing the request and
/// writing it. Everything a provider might object to is objected to
/// afterwards, in a frame — see [`WatchError`], which is the watch that
/// started and then stopped without ending.
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

/// Why there is no change to report, and never will be again.
///
/// What a watch stream carries in
/// [`ResponseStreamError::Payload`](crate::client::response_stream_error::ResponseStreamError::Payload).
/// The stream's own failures — a connection that went, a frame that was
/// not one — are beside it there rather than in here, because they
/// happen to every stream and have nothing to do with watching.
///
/// Both of these end the stream, and both are terminal for the same
/// underlying reason: a filetree is a FOLD. The frames apply to a tree
/// the reader is keeping, so a gap in them leaves that tree permanently
/// wrong with no way to notice. Reading on would be applying changes to
/// something known to be broken. The recovery is a new watch and a
/// fresh snapshot, which is cheap.
#[derive(Debug)]
pub enum WatchError {
    /// The response frame did not parse.
    Response(response::FrameDecodeError),
    /// The provider stopped watching, and said so.
    ///
    /// An answer of a kind — the provider is saying the watch is over
    /// and why — but not a change to the tree, which is why it is not
    /// the stream simply ending.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for WatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WatchError::Response(error) => {
                write!(f, "watch frame did not parse: {error}")
            }
            WatchError::Provider(_) => {
                f.write_str("the provider stopped watching")
            }
        }
    }
}

impl std::error::Error for WatchError {
    /// [`Provider`](WatchError::Provider) has no source, because what
    /// it carries is not a Rust error and deliberately does not
    /// implement one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WatchError::Response(error) => Some(error),
            WatchError::Provider(_) => None,
        }
    }
}
