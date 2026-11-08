//! Finding a provider that holds an image.

use std::sync::Arc;

use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::provider::endpoints::images::has::client::{execute, request};
use diverge_sdk::shared::containers::oci;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;
use tokio::sync::mpsc;

use crate::daemon::Daemon;

/// A provider connected now that can supply `image`, or `None`.
///
/// Every connected provider but `asker` — the provider asking on a
/// run, which has looked in its own Store and its own registries
/// already; `None` when it is the daemon asking for itself, before it
/// chooses a provider for a run, and then nobody is left out — whose
/// capabilities keep a Store or list a registry, the only providers an
/// `images::has` can find anything on, is asked one at once, with the
/// image, references and all, and the first to answer held is the
/// answer. So a provider that pulls from a registry another does not
/// supplies that other through the daemon. The rest are not waited
/// for: the asks go on in a task of their own, and every provider that
/// answers held after is remembered beside the first, so a pull that
/// finds the first gone has the next. A not found, a refusal, a
/// failure and a provider that will not answer in time are all no; no
/// provider answering held is `None`, after the last has answered.
pub async fn locate(daemon: &Arc<Daemon>, image: &Image, asker: Option<&Identity>) -> Option<Identity> {
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
    let daemon = Arc::clone(daemon);
    let image = Arc::new(image.clone());
    tokio::spawn(async move {
        let mut checks: FuturesUnordered<_> = asked
            .into_iter()
            .map(|(identity, handle)| {
                let image = Arc::clone(&image);
                async move { (identity, holds(&handle, &image).await) }
            })
            .collect();
        while let Some((identity, held)) = checks.next().await {
            if held {
                daemon.live.add_holder(&image.digest, identity.clone()).await;
                // Nobody waiting is the first already answered.
                let _ = found.send(identity);
            }
        }
    });
    hits.recv().await
}

/// Whether the provider on `handle` answers held for the image: from
/// the daemon's own images in its Store, or from a registry it pulls
/// from that a reference names — never another identity's images, nor
/// one it holds only because a client supplied it.
async fn holds(handle: &Handle, image: &Image) -> bool {
    let frame = request::Frame(oci::has::request::Request { image: image.clone() });
    matches!(execute::execute(handle, &frame).await, Ok(true))
}
