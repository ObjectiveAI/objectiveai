//! Writing to a terminal.

use super::super::channel_request::Frame;
use crate::shared::containers::terminal::open::Open;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::encode::{Encode, Writer};

/// The way into a terminal while its output streams: each call one
/// channel request, resolving once the proxy has done it.
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

    /// Write `bytes` to the terminal.
    pub async fn stdin(&self, bytes: &[u8]) -> Result<(), SendError> {
        self.send(&Frame::Stdin(bytes)).await
    }

    /// Set the terminal's size.
    pub async fn resize(&self, size: Open) -> Result<(), SendError> {
        self.send(&Frame::Resize(size)).await
    }

    /// Kill the shell: the proxy finishes the scope.
    pub async fn stop(&self) -> Result<(), SendError> {
        self.send(&Frame::Stop).await
    }

    async fn send(&self, frame: &Frame<'_>) -> Result<(), SendError> {
        let mut payload = Vec::new();
        // A size always serializes.
        let _ = frame.encode(&mut Writer::new(&mut payload));
        // The proxy finishes the channel once it has done what was
        // asked: waiting for that is what keeps the next in order.
        let mut channel = self.handle.send_channel_request(self.scope, &payload).await?;
        while channel.response_receiver.recv().await.is_some() {}
        Ok(())
    }
}
