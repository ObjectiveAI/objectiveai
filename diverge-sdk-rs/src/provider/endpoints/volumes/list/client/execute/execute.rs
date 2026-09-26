//! Asking and reading the answer, in one call.

use std::fmt;

use super::super::request;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::volumes::list::server::response;
use crate::wire::frame;
use crate::shared::error::Error;

/// List the volumes a provider offers, and wait for the answer.
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
/// wanted. Every volume endpoint collapses this way.
///
/// # An answer and an error are two different things
///
/// An empty list is an ANSWER: the provider looked and offers
/// nothing. An [`ExecuteError::Provider`] is the absence of one, and
/// a caller that treats them alike concludes there are no volumes
/// when it was never told.
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
/// [`Scope`](crate::wire::client::scope::Scope) is what tells the router
/// nobody is listening for the rest.
pub async fn execute(
    handle: &Handle,
    request: &request::Frame,
) -> Result<Vec<response::Volume>, ExecuteError> {
    let mut payload = Vec::new();
    // Taken as an argument like every other endpoint's, though it has
    // no fields to carry yet, so that all five read alike and a field
    // added later changes nothing here.
    //
    // Its encode is `Infallible`, which is why this error type has no
    // request variant: an empty match on one is how you say there is no
    // value to handle.
    request
        .encode(&mut Writer::new(&mut payload))
        .unwrap_or_else(|error| match error {});
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
        response::Frame::Volumes(volumes) => Ok(volumes),
        response::Frame::Error(error) => Err(ExecuteError::Provider(error)),
    }
}

/// A listing that did not produce an answer.
///
/// All but the last are this end's view of something going wrong. The
/// last is the provider saying so itself, and it is the only one that
/// means the exchange worked.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    ///
    /// Nothing was written, so nothing is waiting to answer it. See
    /// [`SendError`] for the three reasons, only one of which is about
    /// this exchange rather than the whole connection.
    Send(SendError),
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
    Response(response::FrameDecodeError),
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
            ExecuteError::Closed => f.write_str(
                "connection ended before the volume listing answered",
            ),
            ExecuteError::Frame(error) => {
                write!(f, "volume listing answer did not decode: {error}")
            }
            ExecuteError::Unanswered => {
                f.write_str("volume listing finished without an answer")
            }
            ExecuteError::Response(error) => {
                write!(f, "volume listing answer did not parse: {error}")
            }
            ExecuteError::Provider(_) => {
                f.write_str("the provider could not complete the volume listing")
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
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::Provider(_) => None,
        }
    }
}
