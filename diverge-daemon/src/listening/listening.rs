//! The providers' message listens, held while listened to, and merged
//! into one ring.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::provider::endpoints::info::server::response::Frame as Info;
use diverge_sdk::provider::endpoints::messages::listen::client::{execute, request};
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use tokio::sync::{Mutex, watch};

use crate::daemon::{CHANGES_BEHIND, Daemon};

/// How many messages the ring keeps: as many as a listing may fall
/// behind its words by, [`CHANGES_BEHIND`], the one depth every shared
/// source of the daemon's keeps.
const KEPT: usize = CHANGES_BEHIND;

/// Every connected provider's message listen while anything listens,
/// and the ring they feed. One per daemon, in its live state. The
/// rules:
///
/// 1. **Held exactly while listened to.** The first listener in
///    ([`enter`](Self::enter)) opens one listen on every connected
///    provider whose capabilities hold out `messages`; the last out
///    ([`leave`](Self::leave)) drops them all. There is no idle time: a
///    listen held with nobody reading would take messages, and tell
///    their senders they were delivered, for nobody.
/// 2. **One per provider.** While listened to, a provider that
///    connects is given its listen ([`connected`](Self::connected)),
///    and one that disconnects has it dropped, and its lane with it
///    ([`disconnected`](Self::disconnected)).
/// 3. **Lanes.** A provider refuses a second listen on one connection
///    while one is open, so a listen is opened on a provider only once
///    the one before it there has finished: each holds its provider's
///    lane from before its request until its finish is read, and a
///    listen let go cancels and reads to its finish first.
/// 4. **Taken at once.** A listen takes each message the moment it
///    arrives, into the ring, whether or not any listener has read the
///    one before: no listener ever holds a provider back, and the
///    provider tells the sender delivered as the daemon takes it.
/// 5. **The ring.** Every message taken is numbered, from `1`, and
///    kept; past [`KEPT`] the oldest goes. [`latest`](Self::latest) is
///    the newest number, `0` before any.
/// 6. **Each listener at its own pace.** A listener reads the messages
///    after its own number ([`after`](Self::after)), as many as it can
///    send; one whose number fell out of the ring is told how many it
///    missed, and goes on from the oldest kept.
/// 7. **Ends.** A listen refused — the provider relays no messages, or
///    one is open under the daemon's identity already — failed, or
///    finished ends, and is not opened again until it is let go and
///    the next listener's arrives: nothing is retried.
#[derive(Debug)]
pub struct Listening {
    inner: Mutex<Inner>,
    ring: Mutex<Ring>,
    latest: watch::Sender<u64>,
}

impl Default for Listening {
    fn default() -> Self {
        Listening {
            inner: Mutex::new(Inner::default()),
            ring: Mutex::new(Ring::default()),
            latest: watch::channel(0).0,
        }
    }
}

/// What is held, under the one lock.
#[derive(Debug, Default)]
struct Inner {
    /// How many listen now.
    listeners: usize,
    /// One listen per provider, by the provider's identity.
    upstreams: HashMap<Identity, Upstream>,
    /// Each connected provider's lane (rule 3).
    lanes: HashMap<Identity, Arc<Mutex<()>>>,
}

/// One provider's listen, as its task keeps it.
#[derive(Debug)]
struct Upstream {
    /// Dropped, the task cancels the listen and ends.
    _end: watch::Sender<()>,
}

/// The messages kept (rule 5).
#[derive(Debug, Default)]
struct Ring {
    /// The newest number given.
    newest: u64,
    /// The messages kept, oldest first, each with its number.
    kept: VecDeque<(u64, Arc<Kept>)>,
}

/// One message taken: the provider it came through, who sent it, and
/// its body, shared and never copied.
#[derive(Debug)]
pub struct Kept {
    /// The provider it came through.
    pub provider: Identity,
    /// The identity of the sending connection there.
    pub from: Identity,
    /// The body.
    pub body: Bytes,
}

/// One listener of the daemon's messages, from
/// [`enter`](Listening::enter): dropped, it [`leave`](Listening::leave)s.
/// A drop cannot wait on the lock, so the leave is a task of its own.
pub struct Listener {
    /// The daemon whose messages it hears.
    daemon: Arc<Daemon>,
    /// The newest number when it entered: it hears the messages after.
    pub cursor: u64,
}

impl Drop for Listener {
    fn drop(&mut self) {
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let daemon = Arc::clone(&self.daemon);
            runtime.spawn(async move { daemon.live.listening.leave().await });
        }
    }
}

impl Listening {
    /// One listener more (rule 1): the providers' listens opened when
    /// it is the first. Its number is the newest as it entered: it hears
    /// what is taken after.
    pub async fn enter(&self, daemon: &Arc<Daemon>) -> Listener {
        let mut inner = self.inner.lock().await;
        if inner.listeners == 0 {
            for identity in daemon.live.connected_providers().await {
                let (Some(handle), Some(info)) = (daemon.live.provider(&identity).await, daemon.live.capabilities(&identity).await) else {
                    continue;
                };
                open(&mut inner, daemon, identity, handle, &info);
            }
        }
        inner.listeners += 1;
        // Read under the lock, so a listener counted is never handed a
        // number older than one a message it could miss was taken under.
        let cursor = *self.latest.borrow();
        Listener { daemon: Arc::clone(daemon), cursor }
    }

    /// One listener fewer (rule 1): the providers' listens dropped when
    /// it was the last.
    pub async fn leave(&self) {
        let mut inner = self.inner.lock().await;
        inner.listeners = inner.listeners.saturating_sub(1);
        if inner.listeners == 0 {
            inner.upstreams.clear();
        }
    }

    /// A provider connected (rule 2): its listen opened when anything
    /// listens, it holds out `messages`, and none is open on it yet.
    pub async fn connected(&self, daemon: &Arc<Daemon>, identity: Identity, handle: Handle, info: &Info) {
        let mut inner = self.inner.lock().await;
        if inner.listeners > 0 && !inner.upstreams.contains_key(&identity) {
            open(&mut inner, daemon, identity, handle, info);
        }
    }

    /// A provider disconnected (rule 2): its listen and its lane dropped.
    pub async fn disconnected(&self, identity: &Identity) {
        let mut inner = self.inner.lock().await;
        inner.lanes.remove(identity);
        inner.upstreams.remove(identity);
    }

    /// The newest number, to wait on (rule 5).
    pub fn latest(&self) -> watch::Receiver<u64> {
        self.latest.subscribe()
    }

    /// The messages after `cursor` (rule 6): how many it missed, out of
    /// the ring, and every one kept after it, in order.
    pub async fn after(&self, cursor: u64) -> (u64, Vec<(u64, Arc<Kept>)>) {
        let ring = self.ring.lock().await;
        let skipped = match ring.kept.front() {
            Some((oldest, _)) if *oldest > cursor + 1 => *oldest - cursor - 1,
            _ => 0,
        };
        let messages = ring.kept.iter().filter(|(number, _)| *number > cursor).map(|(number, kept)| (*number, Arc::clone(kept))).collect();
        (skipped, messages)
    }

    /// One message taken (rules 4 and 5): numbered, kept, the oldest
    /// let go past [`KEPT`], and the newest number told.
    async fn take(&self, kept: Kept) {
        let newest = {
            let mut ring = self.ring.lock().await;
            ring.newest += 1;
            let number = ring.newest;
            ring.kept.push_back((number, Arc::new(kept)));
            while ring.kept.len() > KEPT {
                ring.kept.pop_front();
            }
            number
        };
        self.latest.send_replace(newest);
    }
}

/// The provider's listen opened on `handle`, its task spawned, and held
/// under `inner`, behind the provider's lane (rule 3) — when the
/// provider holds out `messages`, and nothing otherwise.
fn open(inner: &mut Inner, daemon: &Arc<Daemon>, identity: Identity, handle: Handle, info: &Info) {
    if info.capabilities.messages.is_none() {
        return;
    }
    let (end, ended) = watch::channel(());
    let lane = Arc::clone(inner.lanes.entry(identity.clone()).or_default());
    tokio::spawn(keep(Arc::clone(daemon), identity.clone(), handle, lane, ended));
    inner.upstreams.insert(identity, Upstream { _end: end });
}

/// Keep one provider's listen until it ends or is let go: every message
/// taken into the ring as it arrives (rule 4). Let go — its upstream
/// dropped — the listen is cancelled and read to its finish, every
/// message still arriving taken all the same, before the lane is free
/// (rule 3); let go before the lane was taken, nothing is opened.
async fn keep(daemon: Arc<Daemon>, provider: Identity, handle: Handle, lane: Arc<Mutex<()>>, mut ended: watch::Receiver<()>) {
    let _lane = tokio::select! {
        lane = lane.lock_owned() => lane,
        _ = ended.changed() => return,
    };
    // Rule 7: a listen that does not open ends here.
    let Ok((mut listen, cancel)) = execute::execute(&handle, &request::Frame).await else {
        return;
    };
    let listening = &daemon.live.listening;
    loop {
        tokio::select! {
            item = listen.next() => match item {
                Some(Ok(message)) => listening.take(Kept { provider: provider.clone(), from: message.from, body: message.body }).await,
                // Rule 7: refused, failed, or finished.
                Some(Err(_)) | None => return,
            },
            _ = ended.changed() => {
                cancel.cancel().await;
                while let Some(Ok(message)) = listen.next().await {
                    listening.take(Kept { provider: provider.clone(), from: message.from, body: message.body }).await;
                }
                return;
            }
        }
    }
}
