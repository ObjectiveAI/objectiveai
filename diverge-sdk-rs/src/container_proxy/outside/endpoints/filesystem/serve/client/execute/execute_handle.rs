//! The serve scope, held: asks one at a time, the tree, and the stop.

use bytes::Bytes;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::channel_request;
use super::AskError;
use crate::wire::client::channel::Channel;
use crate::wire::client::handle::Handle;
use crate::container_proxy::outside::endpoints::fuse::mount::server::channel_request as mount;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// A served subtree, as the server holds it: every [`ask`](Self::ask)
/// is one channel, one answer, and [`stop`](Self::stop) is the end.
///
/// Every method takes `&self`, so one handle serves a relay that
/// answers asks on tasks of its own and stops the scope from the
/// loop that heard the stop: the scope's own stream, which carries
/// nothing after the serving but the finish, is kept behind a lock
/// for the stop to read.
#[derive(Debug)]
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
    /// The scope's main stream after the serving: quiet until the
    /// finish that answers the stop, or the connection's end.
    finish: Mutex<UnboundedReceiver<Bytes>>,
}

impl ExecuteHandle {
    pub(super) fn new(handle: Handle, scope: u32, finish: UnboundedReceiver<Bytes>) -> Self {
        ExecuteHandle {
            handle,
            scope,
            finish: Mutex::new(finish),
        }
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// One ask, and its one answer: the bytes of the proxy's channel
    /// response, which are that ask's own frame — a
    /// [`stat`](crate::shared::containers::fuse::stat::response::Frame),
    /// a [`read`](crate::shared::containers::fuse::read::response::Frame),
    /// a [`list`](crate::shared::containers::fuse::list::response::Frame)
    /// or an [`Ack`](crate::shared::containers::fuse::ack::Frame) — for
    /// the server to decode, or to relay as they are. Every path in
    /// an ask is relative to the subtree.
    ///
    /// Asks may be in flight at once: each is its own channel, and the
    /// proxy answers each on its own task.
    pub async fn ask(&self, ask: &mount::Frame<'_>) -> Result<Bytes, AskError> {
        let mut payload = Vec::new();
        channel_request::Frame::Ask(ask.clone())
            .encode(&mut Writer::new(&mut payload))
            .map_err(AskError::Encode)?;
        self.relay(&payload).await
    }

    /// One ask as it arrived from elsewhere, and its one answer as it
    /// comes: `ask` is the payload of a serve channel request — an
    /// ask of a [`containers::serve`](crate::provider::endpoints::containers::serve)
    /// scope, tag and all — sent to the proxy unread, and the bytes
    /// back are the proxy's channel response, unread. What a provider
    /// does for every ask of a container serve.
    pub async fn relay(&self, ask: &[u8]) -> Result<Bytes, AskError> {
        let mut channel = self
            .handle
            .send_channel_request(self.scope, ask)
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

    /// The tree of the subtree as the container holds it, and then
    /// every change in it: the filetree channel, opened, whose
    /// responses are
    /// [`filetree`](crate::container_proxy::outside::endpoints::filesystem::serve::server::channel_response::filetree::Frame)
    /// frames — one snapshot, then one per change — for as long as
    /// the scope lives, and whose finish comes with the scope's. Any
    /// number may be open at once.
    pub async fn filetree(&self) -> Result<Channel, AskError> {
        let mut payload = Vec::new();
        channel_request::Frame::Filetree
            .encode(&mut Writer::new(&mut payload))
            .map_err(AskError::Encode)?;
        self.handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(AskError::Send)
    }

    /// Stop serving: the stop goes out, and this waits for the scope's
    /// finish — the proxy's, after the asks still in flight are
    /// answered and every filetree channel is finished — or the
    /// connection's end. Nothing is answered on the stop's own
    /// channel. A second stop waits for the same finish.
    pub async fn stop(&self) {
        let mut payload = Vec::new();
        channel_request::Frame::Stop
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {
                mount::FrameEncodeError::PathLength(_) => {}
            });
        let _ = self.handle.send_channel_request(self.scope, &payload).await;
        let mut finish = self.finish.lock().await;
        while let Some(bytes) = finish.recv().await {
            if let Ok(frame::server::ServerFrame::ResponseFinish { .. }) = frame::server::ServerFrame::decode(&bytes) {
                return;
            }
        }
    }
}
