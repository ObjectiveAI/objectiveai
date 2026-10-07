//! A provider's listing, asked.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::volumes::list::client::{execute as list_execute, request as list_request};
use diverge_sdk::provider::endpoints::volumes::list::server::response::Volume;
use diverge_sdk::wire::client::handle::Handle;

use super::Fail;
use crate::daemon::Daemon;

/// The handle on the provider, which must be connected now.
pub async fn handle(daemon: &Daemon, identity: &Identity) -> Result<Handle, Fail> {
    daemon
        .live
        .provider(identity)
        .await
        .ok_or_else(|| Fail::Error("the provider is not connected now".to_string()))
}

/// Every volume the provider lists for the daemon, in the order it
/// listed them.
pub async fn list(daemon: &Daemon, identity: &Identity) -> Result<Vec<Volume>, Fail> {
    let handle = handle(daemon, identity).await?;
    match list_execute::execute(&handle, &list_request::Frame).await {
        Ok(volumes) => Ok(volumes),
        Err(list_execute::ExecuteError::Provider(error)) => Err(Fail::refusal(&error)),
        Err(error) => Err(Fail::failed(&error)),
    }
}

/// The volume as its provider lists it, if it does.
pub async fn find(daemon: &Daemon, volume: &reference::Volume) -> Result<Option<Volume>, Fail> {
    Ok(list(daemon, &volume.provider)
        .await?
        .into_iter()
        .find(|listed| listed.name == volume.name))
}
