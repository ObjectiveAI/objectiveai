//! The providers a container may run on.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::client::handle::Handle;
use rand::seq::SliceRandom as _;

use super::capable::{self, Needs};
use crate::daemon::Daemon;
use crate::store::{self, providers_incoming, providers_outgoing};

/// Why no provider could be had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoProvider {
    /// The record pins the container to a provider that is not
    /// connected now.
    NotConnected(Identity),
    /// The record pins the container to a provider whose capabilities
    /// do not hold out what the run needs: what it lacks, in a
    /// sentence.
    Unsupported(Identity, String),
    /// The record pins it to nothing, and no provider is connected.
    None,
    /// The record pins it to nothing, and no connected provider's
    /// capabilities hold out what the run needs: what the last one
    /// asked lacked.
    NoneCapable(String),
}

impl NoProvider {
    /// What a provider's capabilities lacked, where that is why: the
    /// sentence the daemon's `unsupported` error carries.
    pub fn unsupported(&self) -> Option<&str> {
        match self {
            NoProvider::Unsupported(_, what) | NoProvider::NoneCapable(what) => Some(what),
            NoProvider::NotConnected(_) | NoProvider::None => None,
        }
    }
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
            NoProvider::Unsupported(Identity::Outgoing { address }, what) => {
                write!(f, "the outgoing provider at {address} cannot run it: {what}")
            }
            NoProvider::Unsupported(Identity::IncomingUnbrokered { identity }, what) => {
                write!(f, "the incoming provider {identity} cannot run it: {what}")
            }
            NoProvider::None => f.write_str("no provider is connected"),
            NoProvider::NoneCapable(what) => write!(f, "no connected provider can run it: {what}"),
        }
    }
}

/// The providers a container may run on, in the order to try them:
/// the one its record pins it to, which must be connected now and
/// whose capabilities must hold out what the run `needs`, and no
/// other; or, pinned to none, every provider on record that is
/// connected now and holds that out, in random order — so that
/// unpinned containers spread over the providers rather than piling
/// onto the first — and a start that fails on one goes on to the
/// next. A provider's capabilities are its cached `info`, and one
/// that lacks what the run needs is never asked: see
/// [`capable`](super::capable). Nothing connected is
/// [`NoProvider::None`]; nothing capable, [`NoProvider::NoneCapable`].
pub async fn candidates(daemon: &Daemon, pinned: Option<&Identity>, needs: &Needs) -> Result<Vec<(Identity, Handle)>, NoProvider> {
    if let Some(identity) = pinned {
        let Some(handle) = daemon.live.provider(identity).await else {
            return Err(NoProvider::NotConnected(identity.clone()));
        };
        let lacked = daemon.live.capabilities(identity).await.and_then(|info| capable::check(&info, needs).err());
        if let Some(what) = lacked {
            return Err(NoProvider::Unsupported(identity.clone(), what));
        }
        return Ok(vec![(identity.clone(), handle)]);
    }
    let on_record = match on_record(daemon).await {
        Ok(on_record) => on_record,
        Err(_) => Vec::new(),
    };
    let mut connected = Vec::with_capacity(on_record.len());
    let mut lacked = None;
    for identity in on_record {
        let Some(handle) = daemon.live.provider(&identity).await else {
            continue;
        };
        if let Some(what) = daemon.live.capabilities(&identity).await.and_then(|info| capable::check(&info, needs).err()) {
            lacked = Some(what);
            continue;
        }
        connected.push((identity, handle));
    }
    if connected.is_empty() {
        return Err(match lacked {
            Some(what) => NoProvider::NoneCapable(what),
            None => NoProvider::None,
        });
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
