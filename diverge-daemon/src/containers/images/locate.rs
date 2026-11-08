//! Finding a provider that can supply an image.

use std::sync::Arc;

use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::provider::endpoints::images::oci::client::{execute, request};
use diverge_sdk::shared::containers::oci;
use diverge_sdk::shared::containers::request::Image;
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;
use tokio::sync::mpsc;

use super::Pulls;
use crate::daemon::Daemon;

/// A provider connected now that can supply `image`, or `None`; every
/// provider that can is kept in `pulls`, by the scope that said so.
///
/// The rules:
///
/// 1. **Who is asked.** Every connected provider but `asker` — the
///    provider asking on a run, which has looked in its own Store and
///    its own registries already; `None` when it is the daemon asking
///    for itself, and then nobody is left out — whose capabilities
///    keep a Store or list a registry, the only providers that can
///    hold anything.
/// 2. **The scope is the question.** Each is asked by opening an
///    `images::oci` scope with the image, references and all; its
///    first answer is whether it can supply the image. So a provider
///    that pulls from a registry another does not supplies that other
///    through the daemon.
/// 3. **A held scope is kept.** A scope answered held is added to
///    `pulls`, in the order the answers came, and is what the image's
///    manifests, blobs and proof are taken through: no other scope is
///    opened for them. A scope answered otherwise has finished.
/// 4. **The first answers.** The first provider to answer held is the
///    answer. The rest are not waited for: the scopes go on opening in
///    a task of their own, and each later held is kept beside the
///    first, so a pull that finds the first gone has the next.
/// 5. **Everything else is no.** A not found, an unsupported, an error
///    and a scope that would not open are all no; no provider
///    answering held is `None`, after the last has answered.
pub async fn locate(daemon: &Arc<Daemon>, image: &Image, asker: Option<&Identity>, pulls: &Arc<Pulls>) -> Option<Identity> {
    let mut asked = Vec::new();
    for identity in daemon.live.connected_providers().await {
        if asker == Some(&identity) {
            continue;
        }
        let supplies = daemon
            .live
            .capabilities(&identity)
            .await
            .is_some_and(|info| info.capabilities.images_store || !info.capabilities.images_registries.is_empty());
        if !supplies {
            continue;
        }
        if let Some(handle) = daemon.live.provider(&identity).await {
            asked.push((identity, handle));
        }
    }
    if asked.is_empty() {
        return None;
    }
    let (found, mut hits) = mpsc::unbounded_channel();
    let frame = Arc::new(request::Frame(oci::has::request::Request { image: image.clone() }));
    let pulls = Arc::clone(pulls);
    tokio::spawn(async move {
        let mut opening: FuturesUnordered<_> = asked
            .into_iter()
            .map(|(identity, handle)| {
                let frame = Arc::clone(&frame);
                async move { (identity, execute::execute(&handle, &frame).await) }
            })
            .collect();
        while let Some((identity, opened)) = opening.next().await {
            let Ok(scope) = opened else {
                continue;
            };
            if pulls.keep(identity.clone(), Arc::new(scope)).await {
                // Nobody waiting is the first already answered.
                let _ = found.send(identity);
            }
        }
    });
    hits.recv().await
}
