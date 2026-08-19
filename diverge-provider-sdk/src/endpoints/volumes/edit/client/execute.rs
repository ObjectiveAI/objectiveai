//! Asking and reading the answer, in one call.

use std::fmt;

use super::request;
use crate::client::handle::Handle;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::edit::server::response;
use crate::frame;
use crate::shared::error::Error;

/// Change how much a volume reserves, and wait for the answer.
///
/// The rest of this crate describes the exchange; this performs it.
/// Opening a scope, writing the request, waiting for the one frame that
/// answers it, and taking that answer out of two layers of envelope are
/// the same steps every time, and there is nothing in them a caller
/// gets to decide.
///
/// # It collapses into one call because the exchange does
///
/// One question, one answer, then the scope is over — so there is a
/// value to return, and returning it is the whole of what a caller
/// wanted. [`watch`](crate::endpoints::volumes::watch) is the volume
/// endpoint that does not collapse, and its `execute` hands back
/// something to keep reading instead.
///
/// # An answer and an error are two different things
///
/// There is one answer and it carries nothing: the change is made.
/// Everything else is an [`ExecuteError::Provider`], including the
/// case a caller most wants to know about — a volume already holding
/// more than the new reservation allows.
///
/// # Both capacities are one
///
/// This answers once and opens nothing, so one frame of depth is one
/// more than it needs. Zero is not allowed —
/// [`tokio`](tokio::sync::mpsc::channel) panics on it — which is the
/// only reason the second is not zero.
///
/// The scope's request stream is dropped unread. Nothing is supposed to
/// arrive on it, and a provider that opened a channel anyway finds the
/// far end already gone.
///
/// # What ends the wait
///
/// One frame, and there is no timeout here or anywhere else in this
/// protocol. A provider that never answers is waited on until the
/// connection dies, at which point the receiver closes and this returns
/// [`ExecuteError::Closed`].
///
/// The finish frame that follows the answer is never read. This returns
/// as soon as it has one, and dropping the
/// [`Scope`](crate::client::scope::Scope) is what tells the router
/// nobody is listening for the rest.
pub async fn execute(
    handle: &Handle,
    request: &request::Frame,
) -> Result<(), ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let mut scope = handle.send_request(&payload, 1, 1).await;
    let bytes = scope
        .response_receiver
        .recv()
        .await
        .ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes)
        .map_err(ExecuteError::Frame)?;
    // A router puts only a response and its finish on a scope's
    // response stream, so the only other thing this can be is a finish
    // arriving first — a scope that ended without saying anything.
    let frame::server::ServerFrame::Response { payload, .. } = envelope
    else {
        return Err(ExecuteError::Unanswered);
    };
    match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Edited => Ok(()),
        response::Frame::Error(error) => Err(ExecuteError::Provider(error)),
    }
}

/// An edit that did not produce an answer.
///
/// All but the last are this end's view of something going wrong. The
/// last is the provider saying so itself, and it is the only one that
/// means the exchange worked.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request would not serialize.
    Request(postcard::Error),
    /// The connection ended before anything came back.
    ///
    /// Which is the only way waiting stops early. Nothing times a
    /// provider out.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it.
    ///
    /// A provider sends exactly one response before the finish that
    /// ends the scope. One that sends only the finish has said nothing
    /// at all.
    Unanswered,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// The provider could not do it, and said so.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Request(error) => {
                write!(f, "volume edit request did not serialize: {error}")
            }
            ExecuteError::Closed => f.write_str(
                "connection ended before the volume edit answered",
            ),
            ExecuteError::Frame(error) => {
                write!(f, "volume edit answer did not decode: {error}")
            }
            ExecuteError::Unanswered => {
                f.write_str("volume edit finished without an answer")
            }
            ExecuteError::Response(error) => {
                write!(f, "volume edit answer did not parse: {error}")
            }
            ExecuteError::Provider(_) => {
                f.write_str("the provider could not complete the volume edit")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    /// [`Provider`](ExecuteError::Provider) has no source, because what
    /// it carries is not a Rust error and deliberately does not
    /// implement one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::Provider(_) => None,
        }
    }
}
