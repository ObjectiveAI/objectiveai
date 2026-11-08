//! Pulling a manifest, a blob or the proof of an image through the one
//! held source's `images::oci` scope.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::provider::client::Manifest;
use diverge_sdk::shared::containers::oci;
use futures_util::future::ready;
use futures_util::{Stream, StreamExt as _, stream};
use tokio::sync::Mutex;

use super::Source;

/// A blob as it is relayed: the pieces of one source's answer, in
/// order, ending at its finish or short where the source went.
pub type Blob = Pin<Box<dyn Stream<Item = Bytes> + Send>>;

/// The one `images::oci` scope held for an image: the source that
/// answered held first, and the way to it.
///
/// The rules:
///
/// 1. **One source.** A scope is kept here only once its source
///    answered held first, by [`locate`](super::locate); every other
///    was cancelled there. The image's manifests, blobs and proof are
///    taken through this scope and no other, and with none kept each is
///    answered not held.
/// 2. **Let go when answered.** Whoever keeps a source releases it
///    when done — a run once its executor has returned, the container's
///    id or the failure; a client's oci scope once it ends — through
///    [`release`](Self::release): a source holds what it fetched for a
///    scope pinned until the scope ends, so a scope left open would
///    keep it. A source kept after the release is cancelled at once.
/// 3. **A failed ask lets it go.** A source an ask failed on, or that
///    answered a part not held, is cancelled and forgotten, and the ask
///    is answered not held; no other source is asked.
pub struct Pulls {
    state: Mutex<State>,
}

/// What the lock protects.
struct State {
    /// The source held, if any.
    held: Option<Arc<Source>>,
    /// Rule 2: released, and keeping nothing more.
    released: bool,
}

impl Pulls {
    /// No source yet.
    pub fn new() -> Self {
        Pulls {
            state: Mutex::new(State { held: None, released: false }),
        }
    }

    /// Rule 2: the source cancelled and forgotten, and none kept from
    /// now on.
    pub async fn release(&self) {
        let held = {
            let mut state = self.state.lock().await;
            state.released = true;
            state.held.take()
        };
        if let Some(source) = held {
            let _ = source.scope.cancel().await;
        }
    }

    /// Rule 1: `source` kept — `false`, and the source cancelled, once
    /// released (rule 2). A source kept before it is cancelled: one is
    /// held at a time.
    pub async fn keep(&self, source: Source) -> bool {
        let source = Arc::new(source);
        let (kept, replaced) = {
            let mut state = self.state.lock().await;
            if state.released {
                (false, None)
            } else {
                (true, state.held.replace(Arc::clone(&source)))
            }
        };
        if let Some(replaced) = replaced {
            let _ = replaced.scope.cancel().await;
        }
        if !kept {
            let _ = source.scope.cancel().await;
        }
        kept
    }

    /// The source held, if any.
    pub async fn held(&self) -> Option<Arc<Source>> {
        self.state.lock().await.held.clone()
    }

    /// Rule 3: `source` cancelled, and forgotten where it is still the
    /// one held.
    async fn drop_source(&self, source: &Arc<Source>) {
        {
            let mut state = self.state.lock().await;
            if state.held.as_ref().is_some_and(|held| Arc::ptr_eq(held, source)) {
                state.held = None;
            }
        }
        let _ = source.scope.cancel().await;
    }
}

/// The manifest under `digest` through the held source, or `None` when
/// none is held or it does not answer it (rule 3).
pub async fn manifest(pulls: &Pulls, digest: &str) -> Option<Manifest> {
    let source = pulls.held().await?;
    match source.scope.manifest(digest).await {
        Ok(Some(manifest)) => Some(manifest),
        Ok(None) | Err(_) => {
            pulls.drop_source(&source).await;
            None
        }
    }
}

/// The blob under `digest` through the held source, or `None` when
/// none is held or it does not answer it.
///
/// As [`manifest`], except that the source is judged by its first
/// piece: a channel that finishes with none, or fails before any, lets
/// it go (rule 3), and from the first piece on the stream is the
/// source's, ending short where it goes — the asking provider hashes
/// what it got and sees the shortfall, and nothing is retried mid-way.
pub async fn blob(pulls: &Pulls, digest: &str) -> Option<Blob> {
    let source = pulls.held().await?;
    let Ok(mut pieces) = source.scope.blob(digest).await else {
        pulls.drop_source(&source).await;
        return None;
    };
    match pieces.next().await {
        Some(Ok(first)) => {
            let rest = pieces.filter_map(|piece| ready(piece.ok()));
            Some(Box::pin(stream::once(ready(first)).chain(rest)))
        }
        Some(Err(_)) | None => {
            pulls.drop_source(&source).await;
            None
        }
    }
}

/// The proof of possession `request` names, over the held source's
/// image — see [`prove`](diverge_sdk::shared::containers::oci::prove)
/// — or `None` when none is held or it does not answer it (rule 3).
///
/// The daemon hashes nothing itself: the request is sent as it came,
/// platform and nonce, on the source's prove channel, and the proof
/// answered is relayed as it came. No image byte moves through the
/// daemon for it.
pub async fn prove(pulls: &Pulls, request: &oci::prove::request::Request) -> Option<[u8; 32]> {
    let source = pulls.held().await?;
    match source.scope.prove(request).await {
        Ok(Some(proof)) => Some(proof),
        Ok(None) | Err(_) => {
            pulls.drop_source(&source).await;
            None
        }
    }
}
