//! Asking what a provider is.

use std::fmt;
use std::str::Utf8Error;

use super::super::request;
use super::super::super::server::response;
use crate::client::handle::{Handle, SendError};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::frame;

/// Ask a provider its version.
///
/// # It takes no request
///
/// Unlike every other `execute` here, which is handed the frame it is
/// about to send. A version request carries nothing, so there would be
/// nothing to hand over — a parameter whose only possible value is
/// [`request::Frame`] is a parameter that says what the function name
/// already said.
///
/// # It hands back a [`String`], not the frame
///
/// The frame borrows the bytes it was decoded from, and those are gone
/// when this returns. So the version is copied out, once, which is the
/// only copy in the exchange.
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
/// The finish frame that follows the answer is never read either. This
/// returns as soon as it has one, and dropping the
/// [`Scope`](crate::client::scope::Scope) is what tells the router
/// nobody is listening for the rest.
///
/// # What ends the wait
///
/// One frame, and there is no timeout here or anywhere else in this
/// protocol. A provider that never answers is waited on until the
/// connection dies, at which point the receiver closes and this returns
/// [`ExecuteError::Closed`].
///
/// # It fails five ways and none of them is a refusal
///
/// Which is true of nothing else in this crate. Every other answer can
/// come back with the provider saying it could not; a version response
/// has no failure variant, because a provider that could not say what
/// it is could not have received the question. So every one of these is
/// the machinery underneath breaking.
pub async fn execute(handle: &Handle) -> Result<String, ExecuteError> {
    let mut payload = Vec::new();
    request::Frame
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
    let response::Frame(version) =
        response::Frame::decode(payload).map_err(ExecuteError::Response)?;
    Ok(version.to_owned())
}

/// A version request that did not produce an answer.
///
/// All five are this end's view of something going wrong. None is the
/// provider refusing, because a version answer has no way to refuse —
/// see [`execute`].
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    ///
    /// Nothing was written, so nothing is waiting to answer it. See
    /// [`SendError`] for the three reasons, only one of which is about
    /// this exchange rather than the whole connection.
    Send(SendError),
    /// The connection ended before an answer arrived.
    Closed,
    /// The provider finished the scope without answering.
    ///
    /// Distinct from [`Closed`](Self::Closed): the connection is fine
    /// and the provider deliberately said nothing, which is not
    /// something this protocol gives it a way to mean.
    Unanswered,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own
    /// [`Router`](crate::client::router::Router), which decodes the
    /// same bytes before forwarding them and discards what will not
    /// parse.
    Frame(frame::FrameError),
    /// The answer was not UTF-8.
    ///
    /// The payload is the version and nothing else, so this is the
    /// only way reading one can go wrong.
    Response(Utf8Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "the request never went out: {error}")
            }
            ExecuteError::Closed => {
                f.write_str("the connection ended before the version arrived")
            }
            ExecuteError::Unanswered => {
                f.write_str("the scope finished without a version")
            }
            ExecuteError::Frame(error) => {
                write!(f, "version answer did not decode: {error}")
            }
            ExecuteError::Response(error) => {
                write!(f, "version answer was not utf-8: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed | ExecuteError::Unanswered => None,
        }
    }
}
