//! The provider a container runs on.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::client::handle::Handle;

use crate::daemon::Daemon;
use crate::store::{self, providers_incoming, providers_outgoing};

/// Why no provider could be had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoProvider {
    /// The record pins the container to a provider that is not
    /// connected now.
    NotConnected(Identity),
    /// The record pins it to nothing, and no provider is connected.
    None,
}

impl std::fmt::Display for NoProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoProvider::NotConnected(Identity::Outgoing { address }) => {
                write!(f, "the outgoing provider at {address} is not connected now")
            }
            NoProvider::NotConnected(Identity::IncomingUnbrokered { identity }) => {
                write!(f, "the incoming provider {identity} is not connected now")
            }
            NoProvider::None => f.write_str("no provider is connected"),
        }
    }
}

/// The provider for a container: the one its record pins it to, which
/// must be connected now; or, pinned to none, the first connected
/// provider in record order — outgoing by created, then incoming by
/// created — which is "whichever the daemon chooses".
pub async fn choose(daemon: &Daemon, pinned: Option<&Identity>) -> Result<(Identity, Handle), NoProvider> {
    if let Some(identity) = pinned {
        return match daemon.live.provider(identity).await {
            Some(handle) => Ok((identity.clone(), handle)),
            None => Err(NoProvider::NotConnected(identity.clone())),
        };
    }
    let candidates = match on_record(daemon).await {
        Ok(candidates) => candidates,
        Err(_) => Vec::new(),
    };
    for identity in candidates {
        if let Some(handle) = daemon.live.provider(&identity).await {
            return Ok((identity, handle));
        }
    }
    Err(NoProvider::None)
}

/// Every provider on record, outgoing first, each oldest first.
async fn on_record(daemon: &Daemon) -> Result<Vec<Identity>, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let mut identities: Vec<Identity> = providers_outgoing::all(&mut conn)
        .await?
        .into_iter()
        .map(|outgoing| outgoing.identity())
        .collect();
    identities.extend(providers_incoming::all(&mut conn).await?.into_iter().map(|incoming| incoming.provider()));
    Ok(identities)
}
