//! Pulling a manifest or a blob of an image from a provider that can
//! supply it.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::provider::client::Manifest;
use diverge_sdk::provider::endpoints::images::hash;
use diverge_sdk::provider::endpoints::images::oci::client::{execute, request};
use diverge_sdk::shared::containers::oci;
use diverge_sdk::shared::containers::request::Image;
use futures_util::future::ready;
use futures_util::{Stream, StreamExt as _, stream};
use tokio::sync::Mutex;

use super::locate;
use crate::daemon::Daemon;

/// A blob as it is relayed: the pieces of one provider's answer, in
/// order, ending at its finish or short where the provider went.
pub type Blob = Pin<Box<dyn Stream<Item = Bytes> + Send>>;

/// The `images::oci` scopes one run's pull opened, by the provider
/// each is on.
///
/// The rules:
///
/// 1. **One image.** A run asks for one image, so a scope on a holder
///    is opened with that image, at the first manifest or blob asked
///    of the holder, and every later ask of the run reuses it.
/// 2. **Let go when the run is answered.** The start cancels every
///    scope once the run's executor has returned — the container's id
///    or the failure — through [`release`](Self::release): a provider
///    holds what it fetched for a scope pinned until the scope ends,
///    so a scope left open would keep it.
/// 3. **A failed scope is dropped.** A scope an ask failed on is
///    cancelled and forgotten, and the next ask of that holder opens a
///    new one.
pub struct Pulls {
    scopes: Mutex<HashMap<Identity, Arc<execute::ExecuteHandle>>>,
}

impl Pulls {
    /// No scope yet.
    pub fn new() -> Self {
        Pulls { scopes: Mutex::new(HashMap::new()) }
    }

    /// Rule 2: every scope cancelled and forgotten.
    pub async fn release(&self) {
        let scopes: Vec<_> = self.scopes.lock().await.drain().map(|(_, scope)| scope).collect();
        for scope in scopes {
            let _ = scope.cancel().await;
        }
    }

    /// Rule 1: the scope on `holder`, kept or opened now. `None` for a
    /// provider that is not connected now, or whose scope could not be
    /// opened.
    async fn scope(&self, daemon: &Arc<Daemon>, holder: &Identity, image: &Image) -> Option<Arc<execute::ExecuteHandle>> {
        let mut scopes = self.scopes.lock().await;
        if let Some(scope) = scopes.get(holder) {
            return Some(Arc::clone(scope));
        }
        let handle = daemon.live.provider(holder).await?;
        let frame = request::Frame(oci::has::request::Request { image: image.clone() });
        let opened = Arc::new(execute::execute(&handle, &frame).await.ok()?);
        scopes.insert(holder.clone(), Arc::clone(&opened));
        Some(opened)
    }

    /// Rule 3.
    async fn drop_scope(&self, holder: &Identity) {
        let scope = self.scopes.lock().await.remove(holder);
        if let Some(scope) = scope {
            let _ = scope.cancel().await;
        }
    }
}

/// The manifest under `digest`, of `image`, from a provider that can
/// supply the image, or `None` when none does.
///
/// Every holder the daemon knows of is tried in turn, through the
/// run's scope on it, and the first to answer the manifest is the
/// answer. A holder that answers not held is forgotten for the image;
/// one whose scope failed is forgotten and its scope let go; none left
/// is `None`.
pub async fn manifest(daemon: &Arc<Daemon>, pulls: &Pulls, image: &Image, digest: &str, asker: &Identity) -> Option<Manifest> {
    for holder in holders(daemon, image, asker).await {
        let Some(scope) = pulls.scope(daemon, &holder, image).await else {
            daemon.live.drop_holder(&image.digest, &holder).await;
            continue;
        };
        match scope.manifest(digest).await {
            Ok(Some(manifest)) => return Some(manifest),
            Ok(None) => daemon.live.drop_holder(&image.digest, &holder).await,
            Err(_) => failed(daemon, pulls, &image.digest, &holder).await,
        }
    }
    None
}

/// The blob under `digest`, of `image`, from a provider that can
/// supply the image, or `None` when none does.
///
/// As [`manifest`], except that a holder is judged by its first
/// piece: a channel that finishes with none is not held, a channel
/// that fails before any is the scope failed, and from the first
/// piece on the stream is that provider's, ending short where it
/// goes — the asking provider hashes what it got and sees the
/// shortfall, and nothing is retried mid-way.
pub async fn blob(daemon: &Arc<Daemon>, pulls: &Pulls, image: &Image, digest: &str, asker: &Identity) -> Option<Blob> {
    for holder in holders(daemon, image, asker).await {
        let Some(scope) = pulls.scope(daemon, &holder, image).await else {
            daemon.live.drop_holder(&image.digest, &holder).await;
            continue;
        };
        let mut pieces = match scope.blob(digest).await {
            Ok(pieces) => pieces,
            Err(_) => {
                failed(daemon, pulls, &image.digest, &holder).await;
                continue;
            }
        };
        match pieces.next().await {
            Some(Ok(first)) => {
                let rest = pieces.filter_map(|piece| ready(piece.ok()));
                return Some(Box::pin(stream::once(ready(first)).chain(rest)));
            }
            Some(Err(_)) => failed(daemon, pulls, &image.digest, &holder).await,
            None => daemon.live.drop_holder(&image.digest, &holder).await,
        }
    }
    None
}

/// The proof of possession the asking provider's `request` names — see
/// [`prove`](diverge_sdk::shared::containers::oci::prove) — from a
/// provider that can supply the image, or `None` when none does.
///
/// The daemon hashes nothing itself: every holder it knows of is asked,
/// in turn, by [`images::hash`](diverge_sdk::provider::endpoints::images::hash)
/// with the request as it came, image, platform and nonce, and the
/// first proof answered is relayed as it came. No image byte moves
/// through the daemon. A holder that cannot supply the image is
/// forgotten for it; one that cannot be asked, or answers its own
/// error or that it has no source, is passed over.
pub async fn prove(daemon: &Arc<Daemon>, request: &oci::prove::request::Request, asker: &Identity) -> Option<[u8; 32]> {
    let frame = hash::client::request::Frame(request.clone());
    for holder in holders(daemon, &request.image, asker).await {
        let Some(handle) = daemon.live.provider(&holder).await else {
            continue;
        };
        match hash::client::execute::execute(&handle, &frame).await {
            Ok(hash::server::response::Frame::Hash { proof }) => return Some(proof),
            Ok(hash::server::response::Frame::NotFound) => daemon.live.drop_holder(&request.image.digest, &holder).await,
            Ok(_) | Err(_) => {}
        }
    }
    None
}

/// The providers to pull from, in order: those known to supply the
/// image, or, knowing none, the first found by asking — and whoever
/// else answered meanwhile. Never the asker.
async fn holders(daemon: &Arc<Daemon>, image: &Image, asker: &Identity) -> Vec<Identity> {
    let mut known = daemon.live.holders(&image.digest).await;
    if known.is_empty() && locate(daemon, image, Some(asker)).await.is_some() {
        known = daemon.live.holders(&image.digest).await;
    }
    known.retain(|holder| holder != asker);
    known
}

/// A pull from the provider failed: its scope let go and the holder
/// forgotten for the image.
async fn failed(daemon: &Arc<Daemon>, pulls: &Pulls, digest: &str, holder: &Identity) {
    pulls.drop_scope(holder).await;
    daemon.live.drop_holder(digest, holder).await;
}
