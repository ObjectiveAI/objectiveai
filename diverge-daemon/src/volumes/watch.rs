//! Keeping a provider's mirror, for the connection's life.

use std::sync::Arc;

use diverge_sdk::provider::endpoints::volumes::list::client::{execute, request};
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;

use super::Mirror;
use crate::daemon::{Daemon, Kind};

/// Open the provider's `volumes::list` on `handle` and keep the
/// provider's mirror — made and kept by the attach before this runs,
/// so a connection gone meanwhile takes it with its slot and none is
/// kept after — from it: every frame applied, and the word
/// that volumes changed said after each that changed something. A
/// listing that could not open, or that ends in the provider's error,
/// marks the mirror failed; the stream ending — the connection gone —
/// ends this, and the mirror goes with the connection's slot. The
/// listing is opened once per connection and never again, so the
/// provider's one-at-a-time refusal — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time)
/// — is never met; were it met, it marks the mirror failed as an error
/// does, and nothing is retried.
pub async fn watch(daemon: Arc<Daemon>, mirror: Arc<Mirror>, handle: Handle) {
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
