//! Whether the caller holds an image, its manifest and blobs, and
//! the proof that it holds it, from the caller's store.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded;
use super::send::{Stop, finish, respond, respond_pieces};
use crate::provider::client::OciStore;
use crate::wire::client::handle::Handle;
use crate::shared::containers::oci;

/// One frame — not held, or held under a name — then the finish.
pub(crate) async fn has<O: OciStore>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    digest: String,
    store: Arc<O>,
) -> Result<(), Stop> {
    let name = store.holds(&digest).await;
    respond(handle, scope, channel, &oci::has::response::Frame { name }).await?;
    finish(handle, scope, channel).await
}

/// One frame — the manifest's media type and bytes — then the
/// finish; the empty finish for a digest the store does not hold.
pub(crate) async fn manifest<O: OciStore>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    digest: String,
    store: Arc<O>,
) -> Result<(), Stop> {
    if let Some(manifest) = store.manifest(&digest).await {
        let frame = oci::manifest::response::Frame {
            media_type: &manifest.media_type,
            body: &manifest.body,
        };
        respond(handle, scope, channel, &frame).await?;
    }
    finish(handle, scope, channel).await
}

/// One frame — the 32 bytes of the proof — then the finish; the
/// empty finish for an image the store does not hold.
pub(crate) async fn prove<O: OciStore>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    digest: String,
    platform: Option<String>,
    nonce: [u8; 32],
    store: Arc<O>,
) -> Result<(), Stop> {
    if let Some(proof) = store.prove(&digest, platform.as_deref(), &nonce).await {
        respond(handle, scope, channel, &oci::prove::response::Frame { proof }).await?;
    }
    finish(handle, scope, channel).await
}

/// The blob's pieces, each split at the chunk size, then the finish;
/// the empty finish for a digest the store does not hold.
pub(crate) async fn blob<O: OciStore>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    digest: String,
    store: Arc<O>,
) -> Result<(), Stop> {
    if let Some(pieces) = store.blob(&digest).await {
        let mut pieces = std::pin::pin!(pieces);
        while let Some(piece) = pieces.next().await {
            respond_pieces(handle, scope, channel, &piece, |body| {
                encoded(&oci::blob::response::Frame { body })
            })
            .await?;
        }
    }
    finish(handle, scope, channel).await
}
