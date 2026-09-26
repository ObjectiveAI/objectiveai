//! Whether the caller holds an image, and its manifest and blobs,
//! from the caller's store.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded;
use super::send::{Stop, finish, respond, respond_pieces};
use crate::provider::client::OciStore;
use crate::wire::client::handle::Handle;
use crate::shared::containers::oci;

/// One frame — held or not — then the finish.
pub(crate) async fn has<O: OciStore>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    name: String,
    digest: String,
    store: Arc<O>,
) -> Result<(), Stop> {
    let held = store.holds(&name, &digest).await;
    respond(handle, scope, channel, &oci::has::response::Frame { held }).await?;
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
