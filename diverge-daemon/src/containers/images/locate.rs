//! Finding a provider that holds an image.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::images::check::client::{execute, request};
use diverge_sdk::provider::endpoints::images::check::server::response::Response;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;
use tokio::sync::mpsc;

use crate::daemon::Daemon;

/// A provider connected now that holds the image under `digest`, or
/// `None`.
///
/// Every connected provider but `asker` — the one asking, which has
/// looked already — is asked an `images::check` at once, for the
/// daemon's own images there and nothing else, and the first to answer
/// available is the answer. The rest are not waited for: the asks go
/// on in a task of their own, and every provider that answers
/// available after is remembered beside the first, so a pull that
/// finds the first gone has the next. An unavailable, a refusal, a
/// failure and a provider that will not answer in time are all no;
/// no provider answering yes is `None`, after the last has answered.
pub async fn locate(daemon: &Arc<Daemon>, digest: &str, asker: &Identity) -> Option<Identity> {
    let mut asked = Vec::new();
    for identity in daemon.live.connected_providers().await {
        if &identity == asker {
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
    let digest: Arc<str> = Arc::from(digest);
    tokio::spawn(async move {
        let mut checks: FuturesUnordered<_> = asked
            .into_iter()
            .map(|(identity, handle)| {
                let digest = Arc::clone(&digest);
                async move { (identity, holds(&handle, &digest).await) }
            })
            .collect();
        while let Some((identity, held)) = checks.next().await {
            if held {
                daemon.live.add_holder(&digest, identity.clone()).await;
                // Nobody waiting is the first already answered.
                let _ = found.send(identity);
            }
        }
    });
    hits.recv().await
}

/// Whether the provider on `handle` answers available for the digest
/// among the daemon's own images there — the only images a pull from
/// it can deliver, byte for byte, as it fixed them at their build —
/// and not the registries, which are the asking provider's own to
/// pull from; its cache no check can ask, and an image it merely
/// pulled it could not re-serve under the digest.
async fn holds(handle: &Handle, digest: &str) -> bool {
    let frame = request::Frame {
        image: Image {
            digest: digest.to_string(),
            references: Vec::new(),
        },
        sources: request::Sources {
            client: true,
            registries: false,
        },
    };
    matches!(execute::execute(handle, &frame).await, Ok(Response::Available(_)))
}
