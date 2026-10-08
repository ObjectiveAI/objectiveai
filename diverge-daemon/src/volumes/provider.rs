//! A provider's listing, as the daemon mirrors it.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::volumes::list::server::response::Volume;
use diverge_sdk::wire::client::handle::Handle;

use super::Fail;
use crate::daemon::Daemon;

/// The handle on the provider, which must be connected now: what
/// every operation on a volume rides.
pub async fn handle(daemon: &Daemon, identity: &Identity) -> Result<Handle, Fail> {
    daemon
        .live
        .provider(identity)
        .await
        .ok_or_else(|| Fail::Error("the provider is not connected now".to_string()))
}

/// Every volume the provider lists for the daemon, by name: the
/// daemon's mirror of the provider's listing, once the provider has
/// said it is whole. A provider not connected now is the error; one
/// whose listing failed is its error.
pub async fn list(daemon: &Daemon, identity: &Identity) -> Result<Vec<Volume>, Fail> {
    let Some(mirror) = daemon.live.mirror(identity).await else {
        return Err(Fail::Error("the provider is not connected now".to_string()));
    };
    mirror.listed().await?;
    Ok(mirror.volumes().await)
}

/// The volume as its provider lists it, if it does.
pub async fn find(daemon: &Daemon, volume: &reference::Volume) -> Result<Option<Volume>, Fail> {
    Ok(list(daemon, &volume.provider)
        .await?
        .into_iter()
        .find(|listed| listed.name == volume.name))
}
