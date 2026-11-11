//! Writing a frontend request's body.

use super::super::channel_request::Frame;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::encode::{Encode, Writer};

/// The way into a frontend request while its answer streams: each call
/// one channel request, resolving once the proxy has done it.
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

    /// Write a piece of the body; resolves once written.
    pub async fn body(&self, bytes: &[u8]) -> Result<(), SendError> {
        self.send(&Frame::Body(bytes)).await
    }

    /// End the body; resolves once the program's side is told.
    pub async fn end(&self) -> Result<(), SendError> {
        self.send(&Frame::End).await
    }

    /// Drop the request: the proxy finishes the scope.
    pub async fn cancel(&self) -> Result<(), SendError> {
        self.send(&Frame::Cancel).await
    }

    async fn send(&self, frame: &Frame<'_>) -> Result<(), SendError> {
        let mut payload = Vec::new();
        // Nothing can go wrong.
        let _ = frame.encode(&mut Writer::new(&mut payload));
        // The proxy finishes the channel once it has done what was
        // asked: waiting for that is what keeps the next in order.
        let mut channel = self.handle.send_channel_request(self.scope, &payload).await?;
        while channel.response_receiver.recv().await.is_some() {}
        Ok(())
    }
}
