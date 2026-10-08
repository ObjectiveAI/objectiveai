//! Keeping a provider's mirror, for the connection's life.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::volumes::list::client::{execute, request};
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;

use crate::daemon::{Daemon, Kind};

/// Open the provider's `volumes::list` on `handle` and keep the
/// provider's mirror from it: every frame applied, and the word
/// that volumes changed said after each that changed something. A
/// listing that could not open, or that ends in the provider's error,
/// marks the mirror failed; the stream ending — the connection gone —
/// ends this, and the mirror goes with the connection's slot.
pub async fn watch(daemon: Arc<Daemon>, identity: Identity, handle: Handle) {
    let mirror = daemon.live.mirror_provider(identity.clone()).await;
    let (mut listing, _stop) = match execute::execute(&handle, &request::Frame).await {
        Ok(opened) => opened,
        Err(error) => {
            mirror.failed(error.to_string());
            daemon.live.changed(Kind::Volumes);
            return;
        }
    };
    while let Some(item) = listing.next().await {
        let changed = match item {
            Ok(frame) => mirror.apply(frame).await,
            Err(execute::ExecuteStreamError::Refused(error)) => {
                mirror.failed(super::describe(&error));
                true
            }
            Err(error) => {
                mirror.failed(error.to_string());
                true
            }
        };
        if changed {
            daemon.live.changed(Kind::Volumes);
        }
    }
}
