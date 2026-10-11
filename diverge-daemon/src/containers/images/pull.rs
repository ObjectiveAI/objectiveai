//! Pulling a manifest or a blob from a provider that holds the image.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::client::Manifest;
use diverge_sdk::provider::endpoints::images::oci::client::{execute, request};
use futures_util::future::ready;
use futures_util::{Stream, StreamExt as _, stream};

use super::locate;
use crate::daemon::Daemon;

/// A blob as it is relayed: the pieces of one provider's answer, in
/// order, ending at its finish or short where the provider went.
pub type Blob = Pin<Box<dyn Stream<Item = Bytes> + Send>>;

/// The manifest under `digest`, from a provider that holds it, or
/// `None` when none does.
///
/// Every holder the daemon knows of is tried in turn, through the
/// pull scope kept on it — opened now if none is — and the first to
/// answer the manifest is the answer. A holder that answers not
/// held is forgotten for this digest; one whose scope failed is
/// forgotten and its scope let go, to be opened anew by the next
/// pull; none left is `None`.
pub async fn manifest(daemon: &Arc<Daemon>, digest: &str, asker: &Identity) -> Option<Manifest> {
    for holder in holders(daemon, digest, asker).await {
        let Some(scope) = scope(daemon, &holder).await else {
            daemon.live.drop_holder(digest, &holder).await;
            continue;
        };
        match scope.manifest(digest).await {
            Ok(Some(manifest)) => return Some(manifest),
            Ok(None) => daemon.live.drop_holder(digest, &holder).await,
            Err(_) => failed(daemon, digest, &holder).await,
        }
    }
    None
}

/// The blob under `digest`, from a provider that holds it, or `None`
/// when none does.
///
/// As [`manifest`], except that a holder is judged by its first
/// piece: a channel that finishes with none is not held, a channel
/// that fails before any is the scope failed, and from the first
/// piece on the stream is that provider's, ending short where it
/// goes — the asking provider hashes what it got and sees the
/// shortfall, and nothing is retried mid-way.
pub async fn blob(daemon: &Arc<Daemon>, digest: &str, asker: &Identity) -> Option<Blob> {
    for holder in holders(daemon, digest, asker).await {
        let Some(scope) = scope(daemon, &holder).await else {
            daemon.live.drop_holder(digest, &holder).await;
            continue;
        };
        let mut pieces = match scope.blob(digest).await {
            Ok(pieces) => pieces,
            Err(_) => {
                failed(daemon, digest, &holder).await;
                continue;
            }
        };
        match pieces.next().await {
            Some(Ok(first)) => {
                let rest = pieces.filter_map(|piece| ready(piece.ok()));
                return Some(Box::pin(stream::once(ready(first)).chain(rest)));
            }
            Some(Err(_)) => failed(daemon, digest, &holder).await,
            None => daemon.live.drop_holder(digest, &holder).await,
        }
    }
    None
}

/// The providers to pull from, in order: those known to hold the
/// image, or, knowing none, the first found by asking — and whoever
/// else answered meanwhile. Never the asker.
async fn holders(daemon: &Arc<Daemon>, digest: &str, asker: &Identity) -> Vec<Identity> {
    let mut known = daemon.live.holders(digest).await;
    if known.is_empty() && locate(daemon, digest, asker).await.is_some() {
        known = daemon.live.holders(digest).await;
    }
    known.retain(|holder| holder != asker);
    known
}

/// The pull scope on the provider: the one kept, or one opened now
/// and kept — unless another opener kept one first, whose scope is
/// used and this one disconnected. `None` for a provider that is not
/// connected now, or whose scope could not be opened.
async fn scope(daemon: &Arc<Daemon>, holder: &Identity) -> Option<Arc<execute::ExecuteHandle>> {
    if let Some(scope) = daemon.live.pull_scope(holder).await {
        return Some(scope);
    }
    let handle = daemon.live.provider(holder).await?;
    let opened = Arc::new(execute::execute(&handle, &request::Frame).await.ok()?);
    let kept = daemon.live.keep_pull_scope(holder.clone(), Arc::clone(&opened)).await;
    if !Arc::ptr_eq(&kept, &opened) {
        let _ = opened.disconnect().await;
    }
    Some(kept)
}

/// A pull from the provider failed: its scope let go and the holder
/// forgotten for this digest.
async fn failed(daemon: &Arc<Daemon>, digest: &str, holder: &Identity) {
    daemon.live.drop_pull_scope(holder).await;
    daemon.live.drop_holder(digest, holder).await;
}
