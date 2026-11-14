//! The FUSE serve scope, held: asks one at a time, the cancel, and the
//! end.

use bytes::Bytes;
use tokio::sync::{Mutex, OnceCell};

use super::super::channel_request;
use super::AskError;
use crate::wire::client::channel::Channel;
use crate::wire::client::handle::Handle;
use crate::wire::client::scope::Scope;
use crate::container_proxy::outside::endpoints::fuse::mount::server::channel_request as mount;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// A served volume, as the caller holds it: every [`ask`](Self::ask)
/// is one channel, one answer, [`cancel`](Self::cancel) is the end the
/// caller makes, and [`wait`](Self::wait) is the end however it comes.
///
/// The same handle holds a provider's `volumes::fuse_serve` scope and a
/// daemon's `volumes fuse_serve` scope: the two share their channels
/// byte for byte, and differ only in the first answer, which whoever
/// opened the scope has read before this is made.
#[derive(Debug)]
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
    /// The scope's main stream, on which nothing comes after the first
    /// answer but the finish.
    main: Mutex<crate::wire::credit::Receiver>,
    /// How the scope ended, once it has: read once, answered the same
    /// way to everyone who waits.
    end: OnceCell<End>,
}

/// How a FUSE serve scope ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The finish came: the cancel was answered, or the far end ended
    /// the serve for a reason it does not say.
    Finished,
    /// The connection under the scope ended before the finish.
    Closed,
}

impl ExecuteHandle {
    pub(crate) fn new(handle: Handle, scope: Scope) -> Self {
        ExecuteHandle {
            handle,
            scope: scope.scope,
            main: Mutex::new(scope.response_receiver),
            end: OnceCell::new(),
        }
    }

    /// The scope's number, for a caller that keeps its own books.
    pub fn scope(&self) -> u32 {
        self.scope
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
        self.ask_raw(&payload).await
    }

    /// One ask as its bytes already are — a channel request of this
    /// scope that is an ask, encoded — and its one answer, as
    /// [`ask`](Self::ask) gives it. For a relay, which carries an ask
    /// it was sent on to the scope it holds without reading it twice.
    pub async fn ask_raw(&self, payload: &[u8]) -> Result<Bytes, AskError> {
        let mut channel = self
            .handle
            .send_channel_request(self.scope, payload)
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

    /// The tree of the volume as this FUSE serve sees it, and then its
    /// changes: the filetree channel, opened, whose responses are
    /// [`filetree`](crate::provider::endpoints::volumes::fuse_serve::server::channel_response::filetree::Frame)
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

    /// Cancel the serve: the cancel goes out, and this waits for the
    /// scope's end — the far end's finish, after the asks still in
    /// flight are answered, or the connection's end. Nothing is
    /// answered on the cancel's own channel.
    pub async fn cancel(&self) {
        let mut payload = Vec::new();
        channel_request::Frame::Cancel
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {
                mount::FrameEncodeError::PathLength(_) => {}
            });
        let _ = self.handle.send_channel_request(self.scope, &payload).await;
        self.wait().await;
    }

    /// The scope's end, however it comes: the finish — after a cancel,
    /// or because the far end stopped serving on its own — or the
    /// connection under it going first. Nothing else comes on the main
    /// stream after the first answer, and anything that does is passed
    /// over. Read once: whoever waits, and however many, is answered
    /// the same way, during and after.
    ///
    /// A serve that ends before its holder cancelled it is a serve
    /// lost: every later ask on it fails.
    pub async fn wait(&self) -> End {
        *self
            .end
            .get_or_init(|| async {
                let mut main = self.main.lock().await;
                while let Some(bytes) = main.recv().await {
                    match frame::server::ServerFrame::decode(&bytes) {
                        Ok(frame::server::ServerFrame::ResponseFinish { .. }) => return End::Finished,
                        Ok(_) => {}
                        Err(_) => return End::Closed,
                    }
                }
                End::Closed
            })
            .await
    }
}
