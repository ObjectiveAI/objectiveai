//! The mount scope, held for the proxy's life.

use crate::wire::client::handle::{Handle, SendError};

/// The scope of one mount, which the server answers on.
///
/// The server opens no channel on a mount — the two methods here
/// answer the channels the mount opens, by the proxy's own channel
/// numbers. Clones share the scope.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame. Nothing unmounts a mount: its scope
/// lives as long as the proxy does.
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

    /// One answer on a channel the mount opened, by its number.
    pub async fn respond(&self, channel: u32, payload: &[u8]) -> Result<(), SendError> {
        self.handle.send_channel_response(self.scope, channel, payload).await
    }

    /// The end of the answers on a channel the mount opened.
    pub async fn finish(&self, channel: u32) -> Result<(), SendError> {
        self.handle.send_channel_response_finish(self.scope, channel).await
    }
}
