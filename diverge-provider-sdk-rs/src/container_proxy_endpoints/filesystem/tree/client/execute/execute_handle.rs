//! The one thing the server says to a running tree.

use super::super::channel_request;
use crate::client::handle::{Handle, SendError};
use crate::encode::{Encode, Writer};

/// A running tree: the handle and the scope's number, which is all
/// that stopping it needs.
#[derive(Debug, Clone)]
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
}

impl ExecuteHandle {
    pub(super) fn new(handle: Handle, scope: u32) -> Self {
        ExecuteHandle { handle, scope }
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Stop watching. Nothing answers on the channel this opens; what
    /// answers is the scope's own finish, which ends the stream.
    pub async fn stop(&self) -> Result<(), SendError> {
        let mut payload = Vec::new();
        channel_request::Frame
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {});
        self.handle
            .send_channel_request(self.scope, &payload)
            .await
            .map(|_| ())
    }
}
