//! Ending the listing from the caller's side.

use super::super::channel_request;
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// The caller's word that it is done: sends the one channel request
/// of a listing, after which the provider finishes the scope and the
/// [`ExecuteStream`](super::ExecuteStream) ends.
#[derive(Debug, Clone)]
pub struct Stop {
    /// The connection the listing is on.
    handle: Handle,
    /// The listing's scope.
    scope: u32,
}

impl Stop {
    /// For the listing on `scope`.
    pub(super) fn new(handle: Handle, scope: u32) -> Self {
        Stop { handle, scope }
    }

    /// The listing's scope number.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Send the stop. Nothing is answered on its channel; the scope's
    /// finish, which the stream reads as its end, is the answer. A
    /// connection already gone takes the stop nowhere, and the stream
    /// has ended on its own.
    pub async fn stop(&self) {
        let mut payload = Vec::new();
        channel_request::Frame::Stop
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|never| match never {});
        let _ = self.handle.send_channel_request(self.scope, &payload).await;
    }
}
