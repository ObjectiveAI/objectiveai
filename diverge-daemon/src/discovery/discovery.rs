//! The providers' discover listings, held while watched, and merged.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::daemon::endpoints::providers::discover::server::response::{Discovered, Via};
use diverge_sdk::provider::endpoints::discover::client::{execute, request};
use diverge_sdk::provider::endpoints::discover::server::response;
use diverge_sdk::provider::endpoints::discover::server::response::Discoverable;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::discover::Description;
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
///    identity held by a whole listing, with a route for every provider
///    whose listing holds it — the provider, the IP of the identity's
///    connection to it, and the address it dialled the identity at when
///    it did — and every distinct description those listings give it,
///    the daemon's own identity at each provider left out: see
///    [`own_at`](crate::judge::DaemonAuthenticator::own_at).
/// 7. **The routes to an identity.** [`routes`](Self::routes) is a route
///    for every provider whose whole listing holds an identity: what a
///    remote daemon is reached through, as its record's filters let it.
/// 8. **One open at a time.** A provider refuses a second discover
///    scope on one connection while one is open — see
///    [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time)
///    — so a listing is opened on a provider only once the one before it
///    there has finished: each listing holds its provider's lane from
///    before its request until its finish is read, and a listing let go
///    reads to its finish before the lane is free. A listing refused all
///    the same ends as a failure does (rule 4).
/// 9. **Listening.** Each listing's held items keep whether their
///    identity listens for messages there, as the provider's
///    `Listening` and `NotListening` tell it by the identity alone; each
///    route carries it, and a flip is the word [`Kind::Discover`] — the
///    merge then tells it as `Changed`, as every daemon listing tells a
///    change.
#[derive(Debug, Default)]
pub struct Discovery {
    inner: Mutex<Inner>,
    /// A word to [`idle`] that the clock or the watchers changed.
    stirred: Notify,
}

/// One provider's listing as it stands: each identity, as its item
/// lists it — its description, its IP, and the address the provider
/// dialled it at when it did.
type Held = BTreeMap<Identity, Discoverable>;

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
    /// Each connected provider's lane (rule 8): held by the listing
    /// open on it, or still finishing, so the next waits for it. Kept
    /// while discovery is let go, and dropped when the provider
    /// disconnects: a connection after is another connection.
    lanes: HashMap<Identity, Arc<Mutex<()>>>,
}

impl Default for Inner {
    fn default() -> Self {
        Inner { watchers: 0, held: false, touched: Instant::now(), upstreams: HashMap::new(), lanes: HashMap::new() }
    }
}

/// One provider's listing, as its task keeps it.
#[derive(Debug)]
struct Upstream {
    /// The identities it holds now, each with its description.
    held: Arc<Mutex<Held>>,
    /// Whether it has said it is whole, or ended.
    whole: watch::Receiver<bool>,
    /// Dropped, the task stops the listing and ends.
    _end: watch::Sender<()>,
}

/// One watcher of the daemon's discovery, from
/// [`enter`](Discovery::enter): dropped, it [`leave`](Discovery::leave)s.
/// A drop cannot wait on the lock, so the leave is a task of its own.
pub struct Watching {
    /// The daemon whose discovery it watches.
    daemon: Arc<Daemon>,
}

impl Drop for Watching {
    fn drop(&mut self) {
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let daemon = Arc::clone(&self.daemon);
            runtime.spawn(async move { daemon.live.discovery.leave().await });
        }
    }
}

impl Discovery {
    /// One watcher more (rules 1 and 4): discovery held, its listings
    /// opened where they are not, and every listing open waited on until
    /// it is whole. The watcher is the [`Watching`] handed back, counted
    /// from before the wait: dropped however the watch ends — returned,
    /// failed, panicked, its task aborted, even within the wait — it is
    /// matched by exactly one [`leave`](Self::leave).
    pub async fn enter(&self, daemon: &Arc<Daemon>) -> Watching {
        let waiting = self.held(daemon, true).await;
        // Counted with no wait since: nothing between the count and the
        // guard can be cancelled.
        let watching = Watching { daemon: Arc::clone(daemon) };
        self.wait(waiting).await;
        watching
    }

    /// Discovery used to reach a remote daemon (rules 1, 2 and 4): held,
    /// its listings opened where they are not, its clock begun again, and
    /// every listing open waited on until it is whole.
    pub async fn touch(&self, daemon: &Arc<Daemon>) {
        self.hold(daemon, false).await;
    }

    /// Rules 1, 2 and 4, for a watcher entering or a touch.
    async fn hold(&self, daemon: &Arc<Daemon>, watcher: bool) {
        let waiting = self.held(daemon, watcher).await;
        self.wait(waiting).await;
    }

    /// Rules 1 and 4's holding: discovery held, its listings opened
    /// where they are not, and a watcher counted last of all, with no
    /// wait after it; the word of every listing open, to wait on.
    async fn held(&self, daemon: &Arc<Daemon>, watcher: bool) -> Vec<watch::Receiver<bool>> {
        let mut inner = self.inner.lock().await;
        inner.touched = Instant::now();
        if !inner.held {
            inner.held = true;
            for identity in daemon.live.connected_providers().await {
                if let Some(handle) = daemon.live.provider(&identity).await {
                    open(&mut inner, daemon, identity, handle);
                }
            }
        }
        if watcher {
            inner.watchers += 1;
        }
        inner.upstreams.values().map(|upstream| upstream.whole.clone()).collect()
    }

    /// The supervisor stirred, and every listing in `waiting` waited on
    /// until it is whole.
    async fn wait(&self, waiting: Vec<watch::Receiver<bool>>) {
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

    /// Every route to `identity`: each provider whose whole listing holds
    /// it, with where (rule 7).
    pub async fn routes(&self, identity: &Identity) -> Vec<Via> {
        let whole: Vec<(Identity, Arc<Mutex<Held>>)> = {
            let inner = self.inner.lock().await;
            inner
                .upstreams
                .iter()
                .filter(|(_, upstream)| *upstream.whole.borrow())
                .map(|(provider, upstream)| (provider.clone(), Arc::clone(&upstream.held)))
                .collect()
        };
        let mut routes = Vec::new();
        for (provider, held) in whole {
            if let Some(listed) = held.lock().await.get(identity) {
                routes.push(via(&provider, listed));
            }
        }
        routes
    }

    /// A provider connected (rule 3): its listing opened when discovery
    /// is held and none is open on it yet.
    pub async fn connected(&self, daemon: &Arc<Daemon>, identity: Identity, handle: Handle) {
        let mut inner = self.inner.lock().await;
        if inner.held && !inner.upstreams.contains_key(&identity) {
            open(&mut inner, daemon, identity, handle);
        }
    }

    /// A provider disconnected (rules 3, 5 and 8): its listing, what it
    /// told and its lane dropped. Whether there was a listing to drop,
    /// for the word.
    pub async fn disconnected(&self, identity: &Identity) -> bool {
        let mut inner = self.inner.lock().await;
        inner.lanes.remove(identity);
        inner.upstreams.remove(identity).is_some()
    }

    /// The merge (rule 6) of the listings of the providers in `reached`
    /// alone — what a reader's grant reaches, so neither a route nor a
    /// description of another provider's is in it — in order of identity,
    /// each identity's providers in order, and its descriptions in
    /// order, each once.
    pub async fn read(&self, daemon: &Daemon, reached: &HashSet<Identity>) -> Vec<(Identity, Discovered)> {
        let whole: Vec<(Identity, Arc<Mutex<Held>>)> = {
            let inner = self.inner.lock().await;
            inner
                .upstreams
                .iter()
                .filter(|(provider, upstream)| *upstream.whole.borrow() && reached.contains(*provider))
                .map(|(provider, upstream)| (provider.clone(), Arc::clone(&upstream.held)))
                .collect()
        };
        let mut merged: BTreeMap<Identity, (BTreeSet<Via>, BTreeSet<Description>)> = BTreeMap::new();
        for (provider, held) in whole {
            let own = daemon.authenticator.own_at(&provider.issuer);
            for (identity, listed) in held.lock().await.iter() {
                if own.as_ref() == Some(identity) {
                    continue;
                }
                let (providers, descriptions) = merged.entry(identity.clone()).or_default();
                providers.insert(via(&provider, listed));
                descriptions.extend(listed.description.iter().cloned());
            }
        }
        merged
            .into_iter()
            .map(|(identity, (providers, descriptions))| {
                let item = Discovered {
                    issuer: identity.issuer.clone(),
                    subject: identity.subject.clone(),
                    providers: providers.into_iter().collect(),
                    descriptions: descriptions.into_iter().collect(),
                };
                (identity, item)
            })
            .collect()
    }
}

/// The provider's listing opened on `handle`, its task spawned, and
/// held under `inner`, behind the provider's lane (rule 8).
fn open(inner: &mut Inner, daemon: &Arc<Daemon>, identity: Identity, handle: Handle) {
    let held = Arc::new(Mutex::new(BTreeMap::new()));
    let (whole_tx, whole) = watch::channel(false);
    let (end, ended) = watch::channel(());
    let lane = Arc::clone(inner.lanes.entry(identity.clone()).or_default());
    tokio::spawn(keep(Arc::clone(daemon), handle, lane, Arc::clone(&held), whole_tx, ended));
    inner.upstreams.insert(identity, Upstream { held, whole, _end: end });
}

/// Keep one provider's listing until it ends or is let go: each
/// identity added, with its description, or removed, the word said
/// after each; whole at its
/// `listed`, and at its end however it ends. Let go — its upstream
/// dropped — the listing is stopped, so the provider's scope ends too,
/// and read to its finish. The provider's lane is held from before the
/// request until then (rule 8): let go before it was taken, nothing is
/// opened at all.
async fn keep(daemon: Arc<Daemon>, handle: Handle, lane: Arc<Mutex<()>>, held: Arc<Mutex<Held>>, whole: watch::Sender<bool>, mut ended: watch::Receiver<()>) {
    let _lane = tokio::select! {
        lane = lane.lock_owned() => lane,
        _ = ended.changed() => return,
    };
    let (mut listing, cancel) = match execute::execute(&handle, &request::Frame).await {
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
                    Some(Ok(response::Frame::Added(listed))) => {
                        held.lock().await.insert(listed.identity.clone(), listed);
                    }
                    Some(Ok(response::Frame::Removed(listed))) => {
                        held.lock().await.remove(&listed.identity);
                    }
                    Some(Ok(response::Frame::Listed)) => {
                        whole.send_replace(true);
                    }
                    // Rule 9: the listening of an identity held, by its key.
                    Some(Ok(response::Frame::Listening(identity))) => {
                        if let Some(listed) = held.lock().await.get_mut(&identity) {
                            listed.listening = true;
                        }
                    }
                    Some(Ok(response::Frame::NotListening(identity))) => {
                        if let Some(listed) = held.lock().await.get_mut(&identity) {
                            listed.listening = false;
                        }
                    }
                    // Refused (rule 8), failed, or finished: the end.
                    Some(Ok(response::Frame::AlreadyOpen)) | Some(Err(_)) | None => {
                        whole.send_replace(true);
                        daemon.live.changed(Kind::Discover);
                        return;
                    }
                }
                daemon.live.changed(Kind::Discover);
            }
            _ = ended.changed() => {
                // Cancelled, and read to the finish (or the connection's
                // end) before the lane is let go: the provider frees
                // its slot before it sends the finish.
                cancel.cancel().await;
                while let Some(Ok(_)) = listing.next().await {}
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

/// The route to an identity `provider`'s listing holds as `listed`.
fn via(provider: &Identity, listed: &Discoverable) -> Via {
    Via { provider: provider.clone(), ip: listed.ip, address: listed.address.clone(), listening: listed.listening }
}
