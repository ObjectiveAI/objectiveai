//! Starting a watch.

use std::fmt;

use super::super::request;
use super::execute_handle::ExecuteHandle;
use super::execute_stream::ExecuteStream;
use crate::client::handle::{Handle, SendError};
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
/// So it hands back two things. The [`ExecuteStream`] is where
/// everything about reading one lives — what ends it and what a caller
/// owes it — and the [`ExecuteHandle`] is how a caller says stop.
///
/// They are split for the reason
/// [`containers::tools::connect`](crate::endpoints::containers::tools::connect)
/// splits its two: a caller that has stopped reading the changes has
/// not necessarily stopped wanting the watch, and one that wants to end
/// it should not have to hold a stream to do so.
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
/// receiver is unbounded. Holding one unread would mean a provider that
/// opened channels into it grew a queue nobody would ever read, for as
/// long as the watch lasted.
///
/// So it is dropped here and a stray channel request dead-letters,
/// which is the rule [`Scope`](crate::client::scope::Scope) states
/// about itself: drop what you are not going to read.
///
/// # Nothing is built ahead of time
///
/// The disconnect used to be encoded here, because the destructor that
/// sent it could not encode anything itself. There is no destructor
/// now, so [`ExecuteHandle::disconnect`] encodes its own frame when it
/// is called and reports what goes wrong.
pub async fn execute(
    handle: &Handle,
    request: &request::Frame,
) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    Ok((
        ExecuteStream::new(scope.response_receiver),
        ExecuteHandle::new(handle.clone(), scope.scope),
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
    /// The request never went out.
    ///
    /// Nothing was written, so nothing is waiting to answer it. See
    /// [`SendError`] for the three reasons, only one of which is about
    /// this exchange rather than the whole connection.
    Send(SendError),
    /// The request would not serialize.
    Request(postcard::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "the request never went out: {error}")
            }
            ExecuteError::Request(error) => {
                write!(f, "watch request did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
        }
    }
}
