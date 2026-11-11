//! An oci scope held open, and everything a client can ask on it.

use std::fmt;

use tokio::sync::Mutex;

use super::super::channel_request;
use super::answered::{OciBlob, OciManifest, OciProve};
use crate::provider::client::Manifest;
use crate::provider::endpoints::containers::client::{ChannelStream, OpenError, UnaryError, unary};
use crate::shared::containers::oci;
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// A blob's pieces, as the provider sends them, for as long as the
/// channel lives; the finish ends it, and zero pieces before the
/// finish is a digest the scope's image does not reach.
pub type BlobStream = ChannelStream<OciBlob>;

/// The oci scope of one image, answered held, and held until it is
/// cancelled.
///
/// Every channel a client may open on an oci scope is a method here:
/// the three asks, and the cancel. Each opens its own channel, so
/// several may be in flight at once. The readers are the run's: the
/// same shared frames answer the same three asks there.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame. Ending an oci scope is
/// [`cancel`](Self::cancel), or the connection going; the provider
/// finishes the scope on either.
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
    main: Mutex<crate::wire::credit::Receiver>,
}

/// One channel request of this scope, as its bytes.
fn payload(frame: &channel_request::Frame) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes))?;
    Ok(bytes)
}

impl ExecuteHandle {
    pub(crate) fn new(handle: Handle, scope: u32, main: crate::wire::credit::Receiver) -> Self {
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

    /// The manifest under `digest`, of the scope's image: `None` for
    /// the empty finish, a digest the image does not reach.
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
    /// stream that ends with no piece is a digest the image does not
    /// reach.
    pub async fn blob(&self, digest: &str) -> Result<BlobStream, OpenError> {
        let request = oci::blob::request::Request {
            digest: digest.to_string(),
        };
        let payload = payload(&channel_request::Frame::OciBlob(request)).map_err(OpenError::Request)?;
        let channel = self.handle.send_channel_request(self.scope, &payload).await.map_err(OpenError::Send)?;
        Ok(ChannelStream::new(channel.response_receiver))
    }

    /// The proof over the scope's image, under `request`'s platform and
    /// nonce: `None` for the empty finish, a platform that breaks the
    /// rule of [`oci::prove`](crate::shared::containers::oci::prove).
    pub async fn prove(&self, request: &oci::prove::request::Request) -> Result<Option<[u8; 32]>, UnaryError<OciProve>> {
        let payload = payload(&channel_request::Frame::OciProve(request.clone())).map_err(UnaryError::Request)?;
        match unary::<OciProve>(&self.handle, self.scope, &payload).await {
            Ok(proof) => Ok(Some(proof)),
            Err(UnaryError::Unanswered) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// End the scope. Nothing answers on the channel this opens; what
    /// answers is the scope's own finish, on [`wait`](Self::wait).
    pub async fn cancel(&self) -> Result<(), OpenError> {
        let payload = payload(&channel_request::Frame::Cancel).map_err(OpenError::Request)?;
        self.handle.send_channel_request(self.scope, &payload).await.map(|_| ()).map_err(OpenError::Send)
    }

    /// The scope's end: the finish, after the cancel, the provider
    /// ending the scope on its own, or the connection going first. The
    /// held was read by [`execute`](super::execute), and nothing more
    /// comes on the main stream before the finish; anything else that
    /// arrives is passed over. Resolves once and answers the same way
    /// again after.
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

/// How an oci scope answered held ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The finish came: the cancel was answered, or the provider ended
    /// the scope for a reason it does not say.
    Finished,
    /// The connection ended before the finish.
    Closed,
}

impl fmt::Debug for ExecuteHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecuteHandle").field("scope", &self.scope).finish_non_exhaustive()
    }
}
