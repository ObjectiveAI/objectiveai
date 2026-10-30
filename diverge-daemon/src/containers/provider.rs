//! The providers a container may run on.

use std::cmp::Reverse;
use std::sync::Arc;

use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::wire::client::handle::Handle;
use tokio::task::JoinHandle;

use super::capable::{self, Needs};
use super::images;
use crate::daemon::Daemon;

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
    /// sentence, see [`capable::registry`] and [`candidates`].
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
            NoProvider::NotConnected(identity) => write!(f, "the provider {identity} is not connected now"),
            NoProvider::Unsupported(identity, what) => write!(f, "the provider {identity} cannot run it: {what}"),
            NoProvider::None => f.write_str("no provider is connected"),
            NoProvider::NoneCapable(what) => write!(f, "no connected provider can run it: {what}"),
            NoProvider::NoSource { pinned: Some(identity), what } => write!(f, "the provider {identity} cannot source the image: {what}"),
            NoProvider::NoSource { pinned: None, what } => write!(f, "no connected provider can source the image: {what}"),
        }
    }
}

/// The providers a container may run on, in the order to try them:
/// the one its record pins it to, which must be connected now, whose
/// capabilities must hold out what the run `needs`, and which must be
/// able to source the run's image, and no other; or, pinned to none,
/// every provider the configuration accepts that is connected now and passes the
/// same judgements, the one with the most room for the run first —
/// see below — and a start that fails on one goes on to the next.
///
/// # The most room first
///
/// Unpinned, the candidates are sorted by the room their last
/// capacity answer holds for the run — [`capable::headroom`], the
/// scarcer of memory and disk, in runs' worth — greatest first, so
/// that runs land where they fit best and spread as room is used;
/// the record's order breaks ties; a provider that has not answered
/// its capacity yet comes last. A provider short of room by its last
/// answer is sorted after every one with room and is STILL tried:
/// the answer may be up to a poll old, and the provider's own refusal
/// is the one that counts. The pinned path reads no capacity: the
/// one provider is tried whatever its room.
///
/// # Registries before the store, and nothing waited for needlessly
///
/// A provider sources the image from a listed registry a reference
/// names, which the cached info answers at once, or from a client
/// image store that holds it — its own, or another's the daemon
/// supplies from — which only an ask of every store-keeping provider
/// answers. So the ask is SPAWNED first, before any judging, and
/// AWAITED only when it is needed: every connected provider is judged
/// by its capabilities ([`capable::check`]) and then by the registry
/// test ([`capable::registry`]), and when any capable provider
/// passes the registry test, the candidates are exactly those and
/// the ask is left to run out on its own — its answers still fill
/// the daemon's memory of who holds what. Only when no capable
/// provider lists a registry does the chooser wait for the ask, and
/// then every capable provider is a candidate when some store holds
/// the image, or none is. A provider with the image in its own store
/// loses nothing by this: chosen on its registry, it still runs the
/// image from the store, since the provider looks there first.
///
/// # What is refused, and why
///
/// A pinned provider is judged the same way, since it is asked
/// nothing it cannot do: failing its capabilities is
/// [`NoProvider::Unsupported`]; listing no registry, and the ask
/// finding no store with the image, [`NoProvider::NoSource`].
/// Unpinned, nothing connected is [`NoProvider::None`]; nothing left
/// is [`NoProvider::NoSource`] when some capable provider was stopped
/// by the image alone, else [`NoProvider::NoneCapable`].
pub async fn candidates(daemon: &Arc<Daemon>, pinned: Option<&Identity>, needs: &Needs) -> Result<Vec<(Identity, Handle)>, NoProvider> {
    let asked = needs.image.as_ref().map(|image| {
        let daemon = Arc::clone(daemon);
        let digest = image.digest.clone();
        tokio::spawn(async move { images::locate(&daemon, &digest, None).await.is_some() })
    });
    if let Some(identity) = pinned {
        let Some(handle) = daemon.live.provider(identity).await else {
            return Err(NoProvider::NotConnected(identity.clone()));
        };
        match judge(daemon, identity, needs).await {
            Judged::Fit => {}
            Judged::Lacks(what) => return Err(NoProvider::Unsupported(identity.clone(), what)),
            Judged::Unlisted(reason) => {
                if !held(asked).await {
                    return Err(NoProvider::NoSource {
                        pinned: Some(identity.clone()),
                        what: no_source(&reason, needs),
                    });
                }
            }
        }
        return Ok(vec![(identity.clone(), handle)]);
    }
    let on_record = &daemon.providers.accepted;
    let mut listed = Vec::with_capacity(on_record.len());
    let mut unlisted = Vec::new();
    let mut lacked = None;
    let mut reason = None;
    for identity in on_record.iter().cloned() {
        let Some(handle) = daemon.live.provider(&identity).await else {
            continue;
        };
        match judge(daemon, &identity, needs).await {
            Judged::Fit => listed.push((identity, handle)),
            Judged::Lacks(what) => lacked = Some(what),
            Judged::Unlisted(why) => {
                reason = Some(why);
                unlisted.push((identity, handle));
            }
        }
    }
    let candidates = if !listed.is_empty() {
        listed
    } else if !unlisted.is_empty() && held(asked).await {
        unlisted
    } else {
        Vec::new()
    };
    if candidates.is_empty() {
        return Err(match (reason, lacked) {
            (Some(reason), _) => NoProvider::NoSource {
                pinned: None,
                what: no_source(&reason, needs),
            },
            (None, Some(what)) => NoProvider::NoneCapable(what),
            (None, None) => NoProvider::None,
        });
    }
    let mut keyed: Vec<_> = candidates
        .into_iter()
        .map(|(identity, handle)| {
            let key = match daemon.live.capacity(&identity) {
                Some(capacity) => (false, Reverse(capable::headroom(&capacity, needs))),
                None => (true, Reverse(0)),
            };
            (key, identity, handle)
        })
        .collect();
    // Stable: the record's order stands among equals.
    keyed.sort_by_key(|(key, _, _)| *key);
    Ok(keyed.into_iter().map(|(_, identity, handle)| (identity, handle)).collect())
}

/// What one connected provider's judgement came to.
enum Judged {
    /// It holds out everything the run needs and lists a registry
    /// the image's references name — or the run names no image.
    Fit,
    /// Its capabilities lack something the run needs: what.
    Lacks(String),
    /// It holds out everything the run needs, and lists no registry
    /// the image's references name: why, as [`capable::registry`]
    /// says it. Whether a store holds the image decides it.
    Unlisted(String),
}

/// One provider judged, capabilities first and the registry second,
/// from its cached info; a provider whose info the daemon has not
/// cached — not connected, read between two looks — is judged fit
/// and refused by the run instead.
async fn judge(daemon: &Daemon, identity: &Identity, needs: &Needs) -> Judged {
    let Some(info) = daemon.live.capabilities(identity).await else {
        return Judged::Fit;
    };
    if let Err(what) = capable::check(&info, needs) {
        return Judged::Lacks(what);
    }
    if let Some(image) = &needs.image {
        if let Err(why) = capable::registry(&info, image) {
            return Judged::Unlisted(why);
        }
    }
    Judged::Fit
}

/// Whether some connected provider's client image store holds the
/// image: the ask spawned at the chooser's start, waited for here
/// and nowhere else. No ask — the run names no image — and an ask
/// whose task ended without answering are both no.
async fn held(asked: Option<JoinHandle<bool>>) -> bool {
    match asked {
        Some(task) => task.await.unwrap_or(false),
        None => false,
    }
}

/// Why the image cannot be sourced: the registry test's reason, and
/// that no store holds it.
fn no_source(reason: &str, needs: &Needs) -> String {
    let digest = needs.image.as_ref().map(|image| image.digest.as_str()).unwrap_or_default();
    format!("{reason}, and no connected provider holds the image {digest}")
}
