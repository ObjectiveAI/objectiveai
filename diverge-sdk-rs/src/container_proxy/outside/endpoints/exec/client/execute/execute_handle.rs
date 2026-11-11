//! Cancelling an exec.

use super::super::channel_request;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::encode::{Encode, Writer};

/// The way to cancel a command while its output streams.
#[derive(Debug, Clone)]
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
}

impl ExecuteHandle {
    pub(super) fn new(handle: Handle, scope: u32) -> Self {
        ExecuteHandle { handle, scope }
    }

    /// The scope's number.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Cancel the command: the proxy kills it and finishes the scope.
    pub async fn cancel(&self) -> Result<(), SendError> {
        let mut payload = Vec::new();
        channel_request::Frame
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {});
        self.handle.send_channel_request(self.scope, &payload).await.map(|_| ())
    }
}
