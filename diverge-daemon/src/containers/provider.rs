//! The providers a container may run on.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::client::handle::Handle;
use rand::seq::SliceRandom as _;

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

/// The providers a container may run on, in the order to try them:
/// the one its record pins it to, which must be connected now, and no
/// other; or, pinned to none, every provider on record that is
/// connected now, in random order — so that unpinned containers
/// spread over the providers rather than piling onto the first — and
/// a start that fails on one goes on to the next. Nothing connected
/// is [`NoProvider::None`].
pub async fn candidates(daemon: &Daemon, pinned: Option<&Identity>) -> Result<Vec<(Identity, Handle)>, NoProvider> {
    if let Some(identity) = pinned {
        return match daemon.live.provider(identity).await {
            Some(handle) => Ok(vec![(identity.clone(), handle)]),
            None => Err(NoProvider::NotConnected(identity.clone())),
        };
    }
    let on_record = match on_record(daemon).await {
        Ok(on_record) => on_record,
        Err(_) => Vec::new(),
    };
    let mut connected = Vec::with_capacity(on_record.len());
    for identity in on_record {
        if let Some(handle) = daemon.live.provider(&identity).await {
            connected.push((identity, handle));
        }
    }
    if connected.is_empty() {
        return Err(NoProvider::None);
    }
    connected.shuffle(&mut rand::rng());
    Ok(connected)
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
