//! The providers a container may run on.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::client::handle::Handle;
use rand::seq::SliceRandom as _;

use super::capable::{self, Needs};
use super::images;
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
    /// The provider cannot source the run's image — pinned to it, or
    /// pinned to nothing and no capable provider can: why, in a
    /// sentence, see [`capable::image`].
    NoSource {
        /// The provider the record pins the container to, where it
        /// does; `None` where every capable provider was judged.
        pinned: Option<Identity>,
        /// Why the image cannot be sourced.
        what: String,
    },
}

impl NoProvider {
    /// What a provider's capabilities lacked, where that is why: the
    /// sentence the daemon's `unsupported` error carries.
    pub fn unsupported(&self) -> Option<&str> {
        match self {
            NoProvider::Unsupported(_, what) | NoProvider::NoneCapable(what) => Some(what),
            NoProvider::NotConnected(_) | NoProvider::None | NoProvider::NoSource { .. } => None,
        }
    }

    /// Why the run's image cannot be sourced, where that is why: the
    /// sentence the daemon's `no_source` error carries.
    pub fn no_source(&self) -> Option<&str> {
        match self {
            NoProvider::NoSource { what, .. } => Some(what),
            _ => None,
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
            NoProvider::NoSource {
                pinned: Some(Identity::Outgoing { address }),
                what,
            } => write!(f, "the outgoing provider at {address} cannot source the image: {what}"),
            NoProvider::NoSource {
                pinned: Some(Identity::IncomingUnbrokered { identity }),
                what,
            } => write!(f, "the incoming provider {identity} cannot source the image: {what}"),
            NoProvider::NoSource { pinned: None, what } => write!(f, "no connected provider can source the image: {what}"),
        }
    }
}

/// The providers a container may run on, in the order to try them:
/// the one its record pins it to, which must be connected now, whose
/// capabilities must hold out what the run `needs`, and which must be
/// able to source the run's image, and no other; or, pinned to none,
/// every provider on record that is connected now and passes the
/// same two judgements, in random order — so that unpinned
/// containers spread over the providers rather than piling onto the
/// first — and a start that fails on one goes on to the next.
///
/// # The order of judgement
///
/// Where `needs` names an image, whether any connected provider
/// holds it is asked ONCE, of every store-keeping provider at once
/// ([`images::locate`] with nobody left out), before any candidate is
/// judged. Then each provider — the pinned one too, since it is asked
/// nothing it cannot do — is judged by its capabilities first
/// ([`capable::check`]) and by the image second ([`capable::image`]),
/// so that a provider that cannot run the container at all is never
/// reported as merely unable to source the image. A pinned provider
/// that fails the first is [`NoProvider::Unsupported`], the second
/// [`NoProvider::NoSource`]. Unpinned, nothing connected is
/// [`NoProvider::None`]; nothing left once every provider is judged
/// is [`NoProvider::NoSource`] when some capable provider was stopped
/// by the image alone, else [`NoProvider::NoneCapable`].
pub async fn candidates(daemon: &Arc<Daemon>, pinned: Option<&Identity>, needs: &Needs) -> Result<Vec<(Identity, Handle)>, NoProvider> {
    let held = match &needs.image {
        Some(image) => images::locate(daemon, &image.digest, None).await.is_some(),
        None => false,
    };
    if let Some(identity) = pinned {
        let Some(handle) = daemon.live.provider(identity).await else {
            return Err(NoProvider::NotConnected(identity.clone()));
        };
        match judge(daemon, identity, needs, held).await {
            Judged::Fit => {}
            Judged::Lacks(what) => return Err(NoProvider::Unsupported(identity.clone(), what)),
            Judged::Unsourced(what) => {
                return Err(NoProvider::NoSource {
                    pinned: Some(identity.clone()),
                    what,
                });
            }
        }
        return Ok(vec![(identity.clone(), handle)]);
    }
    let on_record = match on_record(daemon).await {
        Ok(on_record) => on_record,
        Err(_) => Vec::new(),
    };
    let mut connected = Vec::with_capacity(on_record.len());
    let mut lacked = None;
    let mut unsourced = None;
    for identity in on_record {
        let Some(handle) = daemon.live.provider(&identity).await else {
            continue;
        };
        match judge(daemon, &identity, needs, held).await {
            Judged::Fit => connected.push((identity, handle)),
            Judged::Lacks(what) => lacked = Some(what),
            Judged::Unsourced(what) => unsourced = Some(what),
        }
    }
    if connected.is_empty() {
        return Err(match (unsourced, lacked) {
            (Some(what), _) => NoProvider::NoSource { pinned: None, what },
            (None, Some(what)) => NoProvider::NoneCapable(what),
            (None, None) => NoProvider::None,
        });
    }
    connected.shuffle(&mut rand::rng());
    Ok(connected)
}

/// What one connected provider's judgement came to.
enum Judged {
    /// It holds out everything the run needs and can source the
    /// image.
    Fit,
    /// Its capabilities lack something the run needs: what.
    Lacks(String),
    /// It holds out everything the run needs, and cannot source the
    /// image: why.
    Unsourced(String),
}

/// One provider judged, capabilities first and the image second,
/// from its cached info; a provider whose info the daemon has not
/// cached — not connected, read between two looks — is judged fit
/// and refused by the run instead.
async fn judge(daemon: &Daemon, identity: &Identity, needs: &Needs, held: bool) -> Judged {
    let Some(info) = daemon.live.capabilities(identity).await else {
        return Judged::Fit;
    };
    if let Err(what) = capable::check(&info, needs) {
        return Judged::Lacks(what);
    }
    if let Some(image) = &needs.image {
        if let Err(what) = capable::image(&info, image, held) {
            return Judged::Unsourced(what);
        }
    }
    Judged::Fit
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
