//! Answering a watch, from a scope and a manager.

use std::pin::pin;

use futures_util::StreamExt as _;
use futures_util::future::{self, Either};

use super::super::response;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::watch::client::request;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Report the tree until somebody stops, then end the scope.
///
/// The one volume handler that does not answer and leave. A snapshot,
/// then a frame per change, for as long as both ends want it.
///
/// # Three things end it, and only one is a failure
///
/// The caller's
/// [`stop`](crate::endpoints::volumes::watch::client::channel_request::Frame),
/// the caller going away, and the manager's stream running out are all
/// an ordinary end: the scope finishes with no error frame, and a
/// caller sees a watch that ended rather than one that broke. An item
/// that is [`Err`] is the failure, and it is reported and then ends the
/// watch too — there is no resuming after one.
///
/// # Why it races rather than reads them in turn
///
/// Because both sides are silent for arbitrarily long. A watch on a
/// quiet tree produces nothing for hours and a caller may stop at any
/// moment during that, so a handler that read the tree first would not
/// notice the stop until something happened to change — which is
/// exactly when it no longer matters.
///
/// [`recv_channel_request`](ScopeHandle::recv_channel_request) is
/// cancel-safe, so the side that loses the race loses nothing.
///
/// # It does not read what the stop SAYS
///
/// The channel request has no payload — a watch's caller has one thing
/// to say and saying it is the whole message. So a frame arriving on
/// any channel of this scope is the stop, and nothing here decodes it.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<M>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    manager: &M,
) where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let stream = match manager.watch(client_identity, &request.name).await {
        Ok(stream) => stream,
        Err(error) => {
            send(&scope, &response::Frame::Error(error.into())).await;
            scope.send_response_finish().await;
            return;
        }
    };
    // The manager's own stream type, pinned here: it need not be
    // `Unpin`, and this is the one place it is read.
    let mut stream = pin!(stream);

    loop {
        // Both futures only borrow the scope, so this races them
        // without taking anything away from the writes below.
        let item = {
            let next = pin!(stream.next());
            let stop = pin!(scope.recv_channel_request());
            match future::select(next, stop).await {
                Either::Left((item, _)) => item,
                // Stopped, or the caller is gone. The same end either
                // way: nothing more is wanted.
                Either::Right(_) => None,
            }
        };

        match item {
            Some(Ok(frame)) => {
                send(&scope, &response::Frame::Filetree(frame)).await
            }
            Some(Err(error)) => {
                send(&scope, &response::Frame::Error(error.into())).await;
                break;
            }
            None => break,
        }
    }

    scope.send_response_finish().await;
}

/// Write one frame, or write nothing if it will not encode.
///
/// The one failure with nowhere to report it: the channel for saying so
/// is the thing that would not serialize.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
