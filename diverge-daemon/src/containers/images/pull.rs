//! Pulling a manifest, a blob or the proof of an image through a
//! provider's held `images::oci` scope.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::provider::client::Manifest;
use diverge_sdk::provider::endpoints::images::oci::client::execute;
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

/// The `images::oci` scopes held for one image, by the provider each
/// is on: the providers that said they can supply it, and the way to
/// it.
///
/// The rules:
///
/// 1. **Held scopes are the holders.** A scope is kept here only once
///    its provider answered held, in the order the answers came, by
///    [`locate`]; the image's manifests, blobs and proof are taken
///    through these scopes and no others.
/// 2. **Let go when the run is answered.** The start releases every
///    scope once the run's executor has returned — the container's id
///    or the failure — through [`release`](Self::release): a provider
///    holds what it fetched for a scope pinned until the scope ends,
///    so a scope left open would keep it. A held answer arriving after
///    the release is cancelled at once.
/// 3. **A failed scope is dropped.** A scope an ask failed on, or that
///    answered a part not held, is cancelled and forgotten, and the
///    next holder is tried.
pub struct Pulls {
    state: Mutex<State>,
}

/// What the lock protects.
struct State {
    /// The held scopes, in the order their providers answered.
    held: Vec<(Identity, Arc<execute::ExecuteHandle>)>,
    /// Rule 2: released, and keeping nothing more.
    released: bool,
}

impl Pulls {
    /// No scope yet.
    pub fn new() -> Self {
        Pulls {
            state: Mutex::new(State { held: Vec::new(), released: false }),
        }
    }

    /// Rule 2: every scope cancelled and forgotten, and none kept from
    /// now on.
    pub async fn release(&self) {
        let held = {
            let mut state = self.state.lock().await;
            state.released = true;
            std::mem::take(&mut state.held)
        };
        for (_, scope) in held {
            let _ = scope.cancel().await;
        }
    }

    /// Rule 1: `scope`, held by `holder`, kept — `false`, and the scope
    /// cancelled, once released (rule 2).
    pub async fn keep(&self, holder: Identity, scope: Arc<execute::ExecuteHandle>) -> bool {
        let kept = {
            let mut state = self.state.lock().await;
            if !state.released {
                state.held.push((holder, Arc::clone(&scope)));
            }
            !state.released
        };
        if !kept {
            let _ = scope.cancel().await;
        }
        kept
    }

    /// The held scopes, in order.
    async fn held(&self) -> Vec<(Identity, Arc<execute::ExecuteHandle>)> {
        self.state.lock().await.held.clone()
    }

    /// Rule 3.
    async fn drop_scope(&self, holder: &Identity) {
        let dropped = {
            let mut state = self.state.lock().await;
            let index = state.held.iter().position(|(held, _)| held == holder);
            index.map(|index| state.held.remove(index).1)
        };
        if let Some(scope) = dropped {
            let _ = scope.cancel().await;
        }
    }
}

/// The manifest under `digest`, of `image`, through a held scope, or
/// `None` when no holder answers it.
///
/// Every held scope is tried in turn, and the first to answer the
/// manifest is the answer; one that answers not held, or fails, is
/// dropped (rule 3).
pub async fn manifest(daemon: &Arc<Daemon>, pulls: &Arc<Pulls>, image: &Image, digest: &str, asker: &Identity) -> Option<Manifest> {
    for (holder, scope) in holders(daemon, pulls, image, asker).await {
        match scope.manifest(digest).await {
            Ok(Some(manifest)) => return Some(manifest),
            Ok(None) | Err(_) => pulls.drop_scope(&holder).await,
        }
    }
    None
}

/// The blob under `digest`, of `image`, through a held scope, or
/// `None` when no holder answers it.
///
/// As [`manifest`], except that a holder is judged by its first
/// piece: a channel that finishes with none, or fails before any, is
/// dropped, and from the first piece on the stream is that provider's,
/// ending short where it goes — the asking provider hashes what it got
/// and sees the shortfall, and nothing is retried mid-way.
pub async fn blob(daemon: &Arc<Daemon>, pulls: &Arc<Pulls>, image: &Image, digest: &str, asker: &Identity) -> Option<Blob> {
    for (holder, scope) in holders(daemon, pulls, image, asker).await {
        let Ok(mut pieces) = scope.blob(digest).await else {
            pulls.drop_scope(&holder).await;
            continue;
        };
        match pieces.next().await {
            Some(Ok(first)) => {
                let rest = pieces.filter_map(|piece| ready(piece.ok()));
                return Some(Box::pin(stream::once(ready(first)).chain(rest)));
            }
            Some(Err(_)) | None => pulls.drop_scope(&holder).await,
        }
    }
    None
}

/// The proof of possession the asking provider's `request` names, over
/// `image` — see [`prove`](diverge_sdk::shared::containers::oci::prove)
/// — through a held scope, or `None` when no holder answers it.
///
/// The daemon hashes nothing itself: the request is sent as it came,
/// platform and nonce, on each held scope's prove channel in turn, and
/// the first proof answered is relayed as it came. No image byte moves
/// through the daemon for it. A holder that answers none, or fails, is
/// dropped (rule 3).
pub async fn prove(daemon: &Arc<Daemon>, pulls: &Arc<Pulls>, image: &Image, request: &oci::prove::request::Request, asker: &Identity) -> Option<[u8; 32]> {
    for (holder, scope) in holders(daemon, pulls, image, asker).await {
        match scope.prove(request).await {
            Ok(Some(proof)) => return Some(proof),
            Ok(None) | Err(_) => pulls.drop_scope(&holder).await,
        }
    }
    None
}

/// The held scopes to take from, in order: those held already, or,
/// none being held — a push, which asks no has first — those
/// [`locate`] opens now. Never the asker's.
async fn holders(daemon: &Arc<Daemon>, pulls: &Arc<Pulls>, image: &Image, asker: &Identity) -> Vec<(Identity, Arc<execute::ExecuteHandle>)> {
    let mut held = pulls.held().await;
    if held.is_empty() && locate(daemon, image, Some(asker), pulls).await.is_some() {
        held = pulls.held().await;
    }
    held.retain(|(holder, _)| holder != asker);
    held
}
