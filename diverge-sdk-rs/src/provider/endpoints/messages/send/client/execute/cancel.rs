//! Ending the send from the caller's side.

use super::super::channel_request;
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// The caller's word that it is done: sends the one channel request
/// of a send, after which the scope's one answer — or the finish with
/// nothing before it, the cancel in time — is what
/// [`Sending::wait`](super::Sending::wait) reads.
#[derive(Debug, Clone)]
pub struct Cancel {
    /// The connection the send is on.
    handle: Handle,
    /// The send's scope.
    scope: u32,
}

impl Cancel {
    /// For the send on `scope`.
    pub(super) fn new(handle: Handle, scope: u32) -> Self {
        Cancel { handle, scope }
    }

    /// The send's scope number.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Send the cancel. Nothing is answered on its channel; the scope's
    /// own answer is. A connection already gone takes the cancel
    /// nowhere, and the scope has ended on its own.
    pub async fn cancel(&self) {
        let mut payload = Vec::new();
        channel_request::Frame::Cancel
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|never| match never {});
        let _ = self.handle.send_channel_request(self.scope, &payload).await;
    }
}
