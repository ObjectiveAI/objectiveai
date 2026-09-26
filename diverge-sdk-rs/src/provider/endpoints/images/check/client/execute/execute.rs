//! Asking the question and reading the answer, in one call.

use std::fmt;

use super::super::request;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::images::check::server::response;
use crate::wire::frame;
use crate::shared::error::Error;

/// Ask whether an image can be supplied, and wait for the answer.
///
/// The rest of this crate describes the exchange; this performs it.
/// Opening a scope, writing the request, waiting for the one frame that
/// answers it, and taking that answer out of two layers of envelope are
/// the same steps every time, and there is nothing in them a caller
/// gets to decide.
///
/// # Why a check gets one of these and not everything does
///
/// Because it is the shape that collapses. One question, one answer,
/// then the scope is over — so there is a value to return, and
/// returning it is the whole of what a caller wanted.
///
/// A laboratory run does not collapse like that. It streams a filetree
/// for as long as its container lives while channels open in both
/// directions, and something that handed back one value would have had
/// to throw most of it away. Those keep the
/// [`Scope`](crate::wire::client::scope::Scope) and read it.
///
/// # A function, not a type
///
/// There is no state to keep between calls. A [`Handle`] is the state,
/// it already exists, and wrapping one in something whose only field is
/// that handle would be a type that exists to be constructed and
/// immediately used.
///
/// A borrow rather than a clone for the same reason. Cloning a
/// [`Handle`] is cheap and would be fine, but nothing here outlives the
/// call, so there is nothing for an owned one to buy.
///
/// # Two answers and an error, which are three things
///
/// [`Available`](crate::provider::endpoints::images::check::server::response::Available)
/// and
/// [`Unavailable`](crate::provider::endpoints::images::check::server::response::Unavailable)
/// are the two ANSWERS. A provider that could not reach one comes back
/// as [`ExecuteError::Provider`], and the difference matters: an
/// unavailable image is a provider that looked, and an error is a
/// provider that could not. Treating them alike either gives up on an
/// image that was there or retries forever against one that never will
/// be.
///
/// # It reads one frame and leaves
///
/// The scope's queues are unbounded and a check takes one thing off one of
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
) -> Result<response::Response, ExecuteError> {
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
        response::Frame::Response(answer) => Ok(answer),
        response::Frame::Error(error) => Err(ExecuteError::Provider(error)),
    }
}

/// A check that did not produce an answer.
///
/// Five of these are this end's view of something going wrong. The
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
    /// The request would not serialize.
    Request(serde_json::Error),
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
    /// ends a check. One that sends only the finish has said nothing at
    /// all, which is different from saying no.
    Unanswered,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// The provider could not answer, and said so.
    ///
    /// **Not an unavailable image.** This is the absence of an answer,
    /// where
    /// [`Unavailable`](crate::provider::endpoints::images::check::server::response::Unavailable)
    /// is one.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "the request never went out: {error}")
            }
            ExecuteError::Request(error) => {
                write!(f, "image check request did not serialize: {error}")
            }
            ExecuteError::Closed => {
                f.write_str("connection ended before the image check answered")
            }
            ExecuteError::Frame(error) => {
                write!(f, "image check answer did not decode: {error}")
            }
            ExecuteError::Unanswered => {
                f.write_str("image check finished without an answer")
            }
            ExecuteError::Response(error) => {
                write!(f, "image check answer did not parse: {error}")
            }
            ExecuteError::Provider(_) => {
                f.write_str("the provider could not answer the image check")
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
            | ExecuteError::Provider(_) => None,
        }
    }
}
