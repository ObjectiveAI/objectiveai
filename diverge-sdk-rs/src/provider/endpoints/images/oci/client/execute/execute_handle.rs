//! A pull held open, and everything a client can ask on it.

use std::fmt;

use bytes::Bytes;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::channel_request;
use super::answered::{OciBlob, OciHas, OciManifest};
use crate::provider::client::Manifest;
use crate::provider::endpoints::containers::client::{ChannelStream, OpenError, UnaryError, unary};
use crate::shared::containers::oci;
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// A blob's pieces, as the provider sends them, for as long as the
/// channel lives; the finish ends it, and zero pieces before the
/// finish is a digest the provider does not hold for the caller.
pub type BlobStream = ChannelStream<OciBlob>;

/// The scope a pull opened, held for the connection's life.
///
/// Every channel a client may open on a pull is a method here: the
/// three asks, and the disconnect. Each opens its own channel, so
/// several may be in flight at once. The readers are the run's: the
/// same shared frames answer the same three asks there.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame. Ending a pull is
/// [`disconnect`](Self::disconnect), or the connection going; the
/// provider finishes the scope on either.
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
    main: Mutex<UnboundedReceiver<Bytes>>,
}

/// One channel request of this scope, as its bytes.
fn payload(frame: &channel_request::Frame) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes))?;
    Ok(bytes)
}

impl ExecuteHandle {
    pub(super) fn new(handle: Handle, scope: u32, main: UnboundedReceiver<Bytes>) -> Self {
        ExecuteHandle {
            handle,
            scope,
            main: Mutex::new(main),
        }
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Whether the provider holds the image for the caller, and under
    /// what repository path: `None` for not held, which the provider
    /// says with the byte `0` or with the empty finish alike.
    pub async fn has(&self, digest: &str) -> Result<Option<String>, UnaryError<OciHas>> {
        let request = oci::has::request::Request {
            digest: digest.to_string(),
        };
        let payload = payload(&channel_request::Frame::OciHas(request)).map_err(UnaryError::Request)?;
        match unary::<OciHas>(&self.handle, self.scope, &payload).await {
            Ok(name) => Ok(name),
            Err(UnaryError::Unanswered) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// The manifest under `digest`, as the provider holds it for the
    /// caller: `None` for the empty finish, a digest not held.
    pub async fn manifest(&self, digest: &str) -> Result<Option<Manifest>, UnaryError<OciManifest>> {
        let request = oci::manifest::request::Request {
            digest: digest.to_string(),
        };
        let payload = payload(&channel_request::Frame::OciManifest(request)).map_err(UnaryError::Request)?;
        match unary::<OciManifest>(&self.handle, self.scope, &payload).await {
            Ok(manifest) => Ok(Some(manifest)),
            Err(UnaryError::Unanswered) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// The blob under `digest`, piece by piece, for as long as the
    /// channel lives: every piece appends, the finish ends it, and a
    /// stream that ends with no piece is a digest not held.
    pub async fn blob(&self, digest: &str) -> Result<BlobStream, OpenError> {
        let request = oci::blob::request::Request {
            digest: digest.to_string(),
        };
        let payload = payload(&channel_request::Frame::OciBlob(request)).map_err(OpenError::Request)?;
        let channel = self.handle.send_channel_request(self.scope, &payload).await.map_err(OpenError::Send)?;
        Ok(ChannelStream::new(channel.response_receiver))
    }

    /// Leave the scope. Nothing answers on the channel this opens;
    /// what answers is the scope's own finish, on [`wait`](Self::wait).
    pub async fn disconnect(&self) -> Result<(), OpenError> {
        let payload = payload(&channel_request::Frame::Disconnect).map_err(OpenError::Request)?;
        self.handle.send_channel_request(self.scope, &payload).await.map(|_| ()).map_err(OpenError::Send)
    }

    /// The scope's end: the finish, after the disconnect or unasked —
    /// a provider that will not serve the pull — or the connection
    /// going first. Resolves once and answers the same way again
    /// after.
    pub async fn wait(&self) -> End {
        let mut main = self.main.lock().await;
        while let Some(bytes) = main.recv().await {
            match frame::server::ServerFrame::decode(&bytes) {
                Ok(frame::server::ServerFrame::ResponseFinish { .. }) => return End::Finished,
                Ok(_) => {}
                Err(_) => return End::Closed,
            }
        }
        End::Closed
    }
}

/// How a pull ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The finish came: the disconnect was answered, or the provider
    /// would not serve the scope.
    Finished,
    /// The connection ended before the finish.
    Closed,
}

impl fmt::Debug for ExecuteHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecuteHandle").field("scope", &self.scope).finish_non_exhaustive()
    }
}
