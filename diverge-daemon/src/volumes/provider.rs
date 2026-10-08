//! A provider's listing, asked.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::volumes::list::client::{execute as list_execute, request as list_request};
use diverge_sdk::provider::endpoints::volumes::list::server::response::{self as list_response, Volume};
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;

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
/// listed them: the provider's listing is a stream kept open, read
/// to the word that it is whole and stopped there.
pub async fn list(daemon: &Daemon, identity: &Identity) -> Result<Vec<Volume>, Fail> {
    let handle = handle(daemon, identity).await?;
    let (mut listing, stop) = list_execute::execute(&handle, &list_request::Frame)
        .await
        .map_err(|error| Fail::failed(&error))?;
    let mut volumes = Vec::new();
    while let Some(item) = listing.next().await {
        match item {
            Ok(list_response::Frame::Added(volume)) => volumes.push(volume),
            Ok(list_response::Frame::Listed) => {
                stop.stop().await;
                return Ok(volumes);
            }
            Ok(_) => {}
            Err(list_execute::ExecuteStreamError::Refused(error)) => return Err(Fail::refusal(&error)),
            Err(error) => return Err(Fail::failed(&error)),
        }
    }
    Err(Fail::Error("the provider's listing ended before it was whole".to_string()))
}

/// The volume as its provider lists it, if it does.
pub async fn find(daemon: &Daemon, volume: &reference::Volume) -> Result<Option<Volume>, Fail> {
    Ok(list(daemon, &volume.provider)
        .await?
        .into_iter()
        .find(|listed| listed.name == volume.name))
}
