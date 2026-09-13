//! Asking and reading the answer, in one call.

use std::fmt;

use super::super::request;
use crate::client::handle::{Handle, SendError};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::create::server::response;
use crate::frame;
use crate::shared::error::Error;

/// Ask a provider to make a volume, and wait for the answer.
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
/// There is one answer and it carries nothing: the volume exists.
/// A size the provider has no room for is
/// [`ExecuteError::InsufficientCapacity`] — the provider's defined
/// refusal, not a failure, and the one a caller acts on by asking
/// smaller. Everything else is an [`ExecuteError::Provider`] — a name
/// already taken, a disk — and this layer names none of them, because
/// a provider knows what happened and this does not.
///
/// # It reads one frame and leaves
///
/// The scope's queues are unbounded and this takes one thing off one of
/// them, so nothing about depth arises. The request stream is dropped
/// unread: nothing is supposed to arrive on it, and a channel a
/// provider opens anyway dead-letters at the router — the provider
/// hears nothing, which is what the wire says of a scope nobody
/// opens channels on.
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
    let mut scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
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
        response::Frame::Created => Ok(()),
        response::Frame::InsufficientCapacity => {
            Err(ExecuteError::InsufficientCapacity)
        }
        response::Frame::Error(error) => Err(ExecuteError::Provider(error)),
    }
}

/// A creation that did not produce an answer.
///
/// All but the last two are this end's view of something going wrong.
/// The last two are the provider saying so itself, and they are the
/// only ones that mean the exchange worked.
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
    /// The provider cannot reserve that many bytes. Nothing exists.
    InsufficientCapacity,
    /// The provider could not do it, and said so.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "the request never went out: {error}")
            }
            ExecuteError::Request(error) => {
                write!(f, "volume creation request did not serialize: {error}")
            }
            ExecuteError::Closed => f.write_str(
                "connection ended before the volume creation answered",
            ),
            ExecuteError::Frame(error) => {
                write!(f, "volume creation answer did not decode: {error}")
            }
            ExecuteError::Unanswered => {
                f.write_str("volume creation finished without an answer")
            }
            ExecuteError::Response(error) => {
                write!(f, "volume creation answer did not parse: {error}")
            }
            ExecuteError::InsufficientCapacity => f.write_str(
                "the provider has insufficient capacity for a volume of that size",
            ),
            ExecuteError::Provider(_) => {
                f.write_str("the provider could not complete the volume creation")
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
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::InsufficientCapacity
            | ExecuteError::Provider(_) => None,
        }
    }
}
