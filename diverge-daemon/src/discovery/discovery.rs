//! The providers' discover listings, held while watched, and merged.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::daemon::endpoints::providers::discover::server::response::Discovered;
use diverge_sdk::provider::endpoints::discover::client::{execute, request};
use diverge_sdk::provider::endpoints::discover::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use tokio::sync::{Mutex, Notify, watch};

use crate::daemon::{Daemon, Kind};

/// Every connected provider's discover listing while it is in use, and
/// their merge. One per daemon, in its live state. The rules:
///
/// 1. **Two modes.** Discovery is held — one listing open on every
///    connected provider — or it is not, and nothing is open. A watcher
///    entering ([`enter`](Self::enter)) or a remote daemon being reached
///    ([`touch`](Self::touch)) holds it, opening the listings where they
///    are not open.
/// 2. **Activity.** It is in use while a watcher is in, and for the
///    daemon's `idle_seconds` after it was last entered, left or
///    touched — the same idle time a container's clock runs. Past that,
///    with no watcher in, [`idle`] stops every listing, and discovery is
///    not held.
/// 2. **Shared.** Every watcher reads the same listings: a provider
///    holds exactly one discover scope of this daemon's at a time,
///    however many watch.
/// 3. **Followed.** While discovery is held, a provider that connects is
///    given its listing ([`connected`](Self::connected)), and one that
///    disconnects has its listing and everything it told dropped
///    ([`disconnected`](Self::disconnected)).
/// 4. **Whole before read.** `enter` and `touch` return once every listing open
///    then has said it is whole, or has ended — a listing that fails,
///    or that the provider does not answer as a listing, counts as
///    whole and holds what it told, which may be nothing — so a
///    watcher's first read holds every provider's.
/// 5. **Told.** Every change a listing tells, its end among them, and
///    every listing dropped, is the word [`Kind::Discover`].
/// 6. **Merged.** [`read`](Self::read) is one [`Discovered`] per
///    identity held by a whole listing, with every provider whose
///    listing holds it, the daemon's own identity at each provider left
///    out: see [`own_at`](crate::judge::DaemonAuthenticator::own_at).
/// 7. **Who holds an identity.** [`providers`](Self::providers) is every
///    provider whose whole listing holds an identity: what a remote
///    daemon is reached through.
#[derive(Debug, Default)]
pub struct Discovery {
    inner: Mutex<Inner>,
    /// A word to [`idle`] that the clock or the watchers changed.
    stirred: Notify,
}

/// What is held, under the one lock.
#[derive(Debug)]
struct Inner {
    /// How many watch now.
    watchers: usize,
    /// Whether discovery is held: every connected provider's listing open.
    held: bool,
    /// When it was last entered, left or touched.
    touched: Instant,
    /// One listing per provider, by the provider's identity.
    upstreams: HashMap<Identity, Upstream>,
}

impl Default for Inner {
    fn default() -> Self {
        Inner { watchers: 0, held: false, touched: Instant::now(), upstreams: HashMap::new() }
    }
}

/// One provider's listing, as its task keeps it.
#[derive(Debug)]
struct Upstream {
    /// The identities it holds now.
    held: Arc<Mutex<BTreeSet<Identity>>>,
    /// Whether it has said it is whole, or ended.
    whole: watch::Receiver<bool>,
    /// Dropped, the task stops the listing and ends.
    _end: watch::Sender<()>,
}

impl Discovery {
    /// One watcher more (rules 1 and 4): discovery held, its listings
    /// opened where they are not, and every listing open waited on until
    /// it is whole. Every `enter` is matched by exactly one
    /// [`leave`](Self::leave).
    pub async fn enter(&self, daemon: &Arc<Daemon>) {
        self.hold(daemon, true).await;
    }

    /// Discovery used to reach a remote daemon (rules 1, 2 and 4): held,
    /// its listings opened where they are not, its clock begun again, and
    /// every listing open waited on until it is whole.
    pub async fn touch(&self, daemon: &Arc<Daemon>) {
        self.hold(daemon, false).await;
    }

    /// Rules 1, 2 and 4, for a watcher entering or a touch.
    async fn hold(&self, daemon: &Arc<Daemon>, watcher: bool) {
        let waiting: Vec<watch::Receiver<bool>> = {
            let mut inner = self.inner.lock().await;
            if watcher {
                inner.watchers += 1;
            }
            inner.touched = Instant::now();
            if !inner.held {
                inner.held = true;
                for identity in daemon.live.connected_providers().await {
                    if let Some(handle) = daemon.live.provider(&identity).await {
                        open(&mut inner, daemon, identity, handle);
                    }
                }
            }
            inner.upstreams.values().map(|upstream| upstream.whole.clone()).collect()
        };
        self.stirred.notify_one();
        for mut whole in waiting {
            let _ = whole.wait_for(|whole| *whole).await;
        }
    }

    /// One watcher fewer (rule 2): its clock begun again, so discovery
    /// stays held for the idle time after the last leaves.
    pub async fn leave(&self) {
        {
            let mut inner = self.inner.lock().await;
            inner.watchers = inner.watchers.saturating_sub(1);
            inner.touched = Instant::now();
        }
        self.stirred.notify_one();
    }

    /// Every provider whose whole listing holds `identity` (rule 7).
    pub async fn providers(&self, identity: &Identity) -> Vec<Identity> {
        let whole: Vec<(Identity, Arc<Mutex<BTreeSet<Identity>>>)> = {
            let inner = self.inner.lock().await;
            inner
                .upstreams
                .iter()
                .filter(|(_, upstream)| *upstream.whole.borrow())
                .map(|(provider, upstream)| (provider.clone(), Arc::clone(&upstream.held)))
                .collect()
        };
        let mut holding = Vec::new();
        for (provider, held) in whole {
            if held.lock().await.contains(identity) {
                holding.push(provider);
            }
        }
        holding
    }

    /// A provider connected (rule 3): its listing opened when discovery
    /// is held and none is open on it yet.
    pub async fn connected(&self, daemon: &Arc<Daemon>, identity: Identity, handle: Handle) {
        let mut inner = self.inner.lock().await;
        if inner.held && !inner.upstreams.contains_key(&identity) {
            open(&mut inner, daemon, identity, handle);
        }
    }

    /// A provider disconnected (rules 3 and 5): its listing and what it
    /// told dropped. Whether there was one to drop, for the word.
    pub async fn disconnected(&self, identity: &Identity) -> bool {
        self.inner.lock().await.upstreams.remove(identity).is_some()
    }

    /// The merge (rule 6), in order of identity, each identity's
    /// providers in order.
    pub async fn read(&self, daemon: &Daemon) -> Vec<(Identity, Discovered)> {
        let whole: Vec<(Identity, Arc<Mutex<BTreeSet<Identity>>>)> = {
            let inner = self.inner.lock().await;
            inner
                .upstreams
                .iter()
                .filter(|(_, upstream)| *upstream.whole.borrow())
                .map(|(provider, upstream)| (provider.clone(), Arc::clone(&upstream.held)))
                .collect()
        };
        let mut merged: BTreeMap<Identity, BTreeSet<Identity>> = BTreeMap::new();
        for (provider, held) in whole {
            let own = daemon.authenticator.own_at(&provider.issuer);
            for identity in held.lock().await.iter() {
                if own.as_ref() == Some(identity) {
                    continue;
                }
                merged.entry(identity.clone()).or_default().insert(provider.clone());
            }
        }
        merged
            .into_iter()
            .map(|(identity, providers)| {
                let item = Discovered {
                    issuer: identity.issuer.clone(),
                    subject: identity.subject.clone(),
                    providers: providers.into_iter().collect(),
                };
                (identity, item)
            })
            .collect()
    }
}

/// The provider's listing opened on `handle`, its task spawned, and
/// held under `inner`.
fn open(inner: &mut Inner, daemon: &Arc<Daemon>, identity: Identity, handle: Handle) {
    let held = Arc::new(Mutex::new(BTreeSet::new()));
    let (whole_tx, whole) = watch::channel(false);
    let (end, ended) = watch::channel(());
    tokio::spawn(keep(Arc::clone(daemon), handle, Arc::clone(&held), whole_tx, ended));
    inner.upstreams.insert(identity, Upstream { held, whole, _end: end });
}

/// Keep one provider's listing until it ends or is let go: each
/// identity added or removed, the word said after each; whole at its
/// `listed`, and at its end however it ends. Let go — its upstream
/// dropped — the listing is stopped, so the provider's scope ends too.
async fn keep(daemon: Arc<Daemon>, handle: Handle, held: Arc<Mutex<BTreeSet<Identity>>>, whole: watch::Sender<bool>, mut ended: watch::Receiver<()>) {
    let (mut listing, stop) = match execute::execute(&handle, &request::Frame).await {
        Ok(opened) => opened,
        Err(_) => {
            whole.send_replace(true);
            daemon.live.changed(Kind::Discover);
            return;
        }
    };
    loop {
        tokio::select! {
            item = listing.next() => {
                match item {
                    Some(Ok(response::Frame::Added(identity))) => {
                        held.lock().await.insert(identity);
                    }
                    Some(Ok(response::Frame::Removed(identity))) => {
                        held.lock().await.remove(&identity);
                    }
                    Some(Ok(response::Frame::Listed)) => {
                        whole.send_replace(true);
                    }
                    Some(Err(_)) | None => {
                        whole.send_replace(true);
                        daemon.live.changed(Kind::Discover);
                        return;
                    }
                }
                daemon.live.changed(Kind::Discover);
            }
            _ = ended.changed() => {
                stop.stop().await;
                return;
            }
        }
    }
}

/// Discovery's clock (rule 2), for the daemon's life: while it is held
/// and no watcher is in, it waits out the idle time since it was last
/// entered, left or touched, and then stops every listing — discovery
/// not held. Any change of the clock or the watchers wakes it to wait
/// again. No polling: it sleeps to the deadline or the word.
pub async fn idle(daemon: Arc<Daemon>) {
    let discovery = &daemon.live.discovery;
    loop {
        let deadline = {
            let inner = discovery.inner.lock().await;
            (inner.held && inner.watchers == 0).then(|| inner.touched + daemon.idle)
        };
        let Some(deadline) = deadline else {
            discovery.stirred.notified().await;
            continue;
        };
        tokio::select! {
            () = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                let mut inner = discovery.inner.lock().await;
                if inner.held && inner.watchers == 0 && inner.touched + daemon.idle <= Instant::now() {
                    inner.upstreams.clear();
                    inner.held = false;
                    drop(inner);
                    daemon.live.changed(Kind::Discover);
                }
            }
            () = discovery.stirred.notified() => {}
        }
    }
}
