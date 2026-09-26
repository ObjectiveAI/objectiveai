//! The serve scope, held: asks one at a time, and the stop.

use bytes::Bytes;

use super::super::channel_request;
use super::AskError;
use crate::wire::client::handle::Handle;
use crate::wire::client::scope::Scope;
use crate::container_proxy::outside::fuse::mount::server::channel_request as mount;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// A served volume, as the caller holds it: every [`ask`](Self::ask)
/// is one channel, one answer, and [`stop`](Self::stop) is the end.
#[derive(Debug)]
pub struct ExecuteHandle {
    handle: Handle,
    scope: Scope,
}

impl ExecuteHandle {
    pub(super) fn new(handle: Handle, scope: Scope) -> Self {
        ExecuteHandle { handle, scope }
    }

    /// The scope's number, for a caller that keeps its own books.
    pub fn scope(&self) -> u32 {
        self.scope.scope
    }

    /// One ask, and its one answer: the bytes of the provider's
    /// channel response, which are that ask's own frame — a
    /// [`stat`](crate::shared::containers::fuse::stat::response::Frame),
    /// a [`read`](crate::shared::containers::fuse::read::response::Frame),
    /// a [`list`](crate::shared::containers::fuse::list::response::Frame)
    /// or an [`Ack`](crate::shared::containers::fuse::ack::Frame) — for
    /// the caller to decode, or to relay as they are. A bridge relays
    /// them as they are.
    ///
    /// Asks may be in flight at once: each is its own channel, and the
    /// provider answers each on its own task.
    pub async fn ask(&self, ask: &mount::Frame<'_>) -> Result<Bytes, AskError> {
        let mut payload = Vec::new();
        channel_request::Frame::Ask(ask.clone())
            .encode(&mut Writer::new(&mut payload))
            .map_err(AskError::Encode)?;
        let mut channel = self
            .handle
            .send_channel_request(self.scope.scope, &payload)
            .await
            .map_err(AskError::Send)?;
        loop {
            let bytes = channel.response_receiver.recv().await.ok_or(AskError::Closed)?;
            match frame::server::ServerFrame::decode(&bytes) {
                Ok(frame::server::ServerFrame::ChannelResponse { payload, .. }) => {
                    return Ok(bytes.slice_ref(payload));
                }
                Ok(frame::server::ServerFrame::ChannelResponseFinish { .. }) => return Err(AskError::Unserved),
                _ => {}
            }
        }
    }

    /// Stop serving: the stop goes out, and this waits for the scope's
    /// finish — the provider's, after the asks still in flight are
    /// answered — or the connection's end. Nothing is answered on the
    /// stop's own channel.
    pub async fn stop(mut self) {
        let mut payload = Vec::new();
        channel_request::Frame::Stop
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {
                mount::FrameEncodeError::PathLength(_) => {}
            });
        let _ = self.handle.send_channel_request(self.scope.scope, &payload).await;
        while let Some(bytes) = self.scope.response_receiver.recv().await {
            if let Ok(frame::server::ServerFrame::ResponseFinish { .. }) = frame::server::ServerFrame::decode(&bytes) {
                return;
            }
        }
    }
}
