//! An accept scope, held.

use std::fmt;

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::channel_request;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// The scope an accept opened, held for as long as the daemon accepts:
/// the identity the provider knows the daemon by, the stop, and the
/// end.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame. Ending an accept is
/// [`stop`](Self::stop), or the connection going; the task answering
/// the provider's announcements runs until the scope's request stream
/// ends either way.
pub struct Accepting {
    /// The identity the provider authorized the daemon's connection
    /// under: what connectors name it by.
    pub identity: String,
    handle: Handle,
    scope: u32,
    responses: UnboundedReceiver<Bytes>,
}

impl Accepting {
    pub(super) fn new(identity: String, handle: Handle, scope: u32, responses: UnboundedReceiver<Bytes>) -> Self {
        Accepting {
            identity,
            handle,
            scope,
            responses,
        }
    }

    /// Stop accepting. Nothing answers on the channel this opens; what
    /// answers is the scope's own finish, on [`wait`](Self::wait).
    pub async fn stop(&self) -> Result<(), SendError> {
        let mut payload = Vec::new();
        channel_request::Frame::Stop
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {});
        self.handle.send_channel_request(self.scope, &payload).await.map(|_| ())
    }

    /// The accept's end: the finish, after the stop or at the
    /// provider's own ending, or the connection going first.
    pub async fn wait(&mut self) -> End {
        while let Some(bytes) = self.responses.recv().await {
            match frame::server::ServerFrame::decode(&bytes) {
                Ok(frame::server::ServerFrame::ResponseFinish { .. }) => return End::Finished,
                Ok(_) => {}
                Err(_) => return End::Closed,
            }
        }
        End::Closed
    }
}

/// How an accept ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The finish came: the stop was answered, or the provider ended
    /// the accept.
    Finished,
    /// The connection ended before the finish.
    Closed,
}

impl fmt::Debug for Accepting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Accepting")
            .field("identity", &self.identity)
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}
