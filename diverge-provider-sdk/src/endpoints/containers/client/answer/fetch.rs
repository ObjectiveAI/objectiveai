//! Mounted content the provider does not hold, from the caller's
//! store.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded;
use super::send::{Stop, finish, respond_pieces};
use crate::client::IdentityStore;
use crate::client::handle::Handle;
use crate::shared::containers::{fetch_directory, fetch_file};

/// The file's pieces, each split at the chunk size, then the finish;
/// the empty finish for an identity the store does not hold.
pub(crate) async fn file<I: IdentityStore>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    identity: String,
    store: Arc<I>,
) -> Result<(), Stop> {
    if let Some(mut pieces) = store.file(&identity).await {
        while let Some(piece) = pieces.next().await {
            respond_pieces(handle, scope, channel, &piece, |body| {
                encoded(&fetch_file::response::Frame { body })
            })
            .await?;
        }
    }
    finish(handle, scope, channel).await
}

/// Every file of the directory, each piece under its path and split
/// at the chunk size — adjacent frames with an equal path are one
/// file — then the finish; the empty finish for an identity the
/// store does not hold.
pub(crate) async fn directory<I: IdentityStore>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    identity: String,
    store: Arc<I>,
) -> Result<(), Stop> {
    if let Some(mut files) = store.directory(&identity).await {
        while let Some((path, piece)) = files.next().await {
            respond_pieces(handle, scope, channel, &piece, |body| {
                encoded(&fetch_directory::response::Frame {
                    path: path.clone(),
                    body,
                })
            })
            .await?;
        }
    }
    finish(handle, scope, channel).await
}
