//! What is live: nobody's record, gone at restart.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use arc_swap::ArcSwap;
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use diverge_sdk::daemon::endpoints::providers::Status;
use diverge_sdk::provider::endpoints::capacity::server::response::Frame as CapacityFrame;
use diverge_sdk::provider::endpoints::info::server::response::Frame as Info;
use diverge_sdk::wire::client::handle::Handle;
use tokio::sync::{Mutex, watch};
use tokio::task::AbortHandle;

use chrono::{DateTime, Utc};

use diverge_sdk::daemon::reference;

use super::{Capacity, Changes, Kind, Peers, Standing};
use crate::containers::services::State;
use crate::commands::Commander;
use crate::containers::lifecycle::RunLog;
use crate::containers::message::Waiting;
use crate::containers::{AgentRun, ConnectionId, Key, ToolKey, ToolRun};
use crate::daemons::Peer;
use crate::judge::AgentState;
use crate::database::Scope;
use crate::store::agents::Agent;
use crate::store::{self, AccountId, AgentId, Store};
use crate::listings::Listings;
use crate::listening::Listening;
use crate::messaging::Messaging;
use crate::providers::{Features, Source};
use crate::volumes::Mirror;

/// The live state of the daemon: which accounts have a client
/// connected as them, and how many; which providers the daemon holds
/// a connection to, the handle it speaks to each on, the info each
/// answered and the capacity each last answered, how each connection
/// was made and what the daemon holds open on it; and the dial
/// tasks keeping the ones it dials connected; which remote daemons it
/// holds a connection to through them; which providers hold which image,
/// and the pull scope held on each; every agent's log as
/// it is appended and watched; which agents and tools run — a tool
/// on record, or a dependency tool — and what is
/// live for each; and every agent's outbox, the messages admitted for
/// it and not yet sent. Behind async mutexes, held for a lookup and never
/// across anything that waits on the outside.
#[derive(Default)]
pub struct Live {
    /// Connections per account, for the accounts with any.
    connected: Mutex<HashMap<AccountId, usize>>,
    /// Every client connection's closer, by whom it is connected as,
    /// each by the key its connection was given: what
    /// [`disconnect`](Self::disconnect) tells.
    closers: Mutex<HashMap<Connected, HashMap<u64, watch::Sender<bool>>>>,
    /// The key the next closer is given.
    next_closer: Mutex<u64>,
    /// The administrators connected now, by identity, and how many
    /// connections each: they have no account to count by.
    admins_connected: Mutex<HashMap<Identity, usize>>,
    /// When each administrator's connections last began or all ended,
    /// since the daemon started: they have no row to keep it in.
    admins_last_connected: Mutex<HashMap<Identity, DateTime<Utc>>>,
    /// The tools working now — a call or a read in flight — as their
    /// logs mark them `active`; a tool leaves when it ceases, when its
    /// run ends, and when it is deleted.
    busy_tools: Mutex<HashSet<ToolKey>>,
    /// The providers connected now, one slot per identity, and the
    /// caller's handle on each: what every request to a provider
    /// rides. See [`Peers`].
    providers: Mutex<Peers>,
    /// The current dial of every provider the daemon dials, by its
    /// identity: the number of the one dial task that keeps dialling
    /// it. A task whose number is no longer here ends at its next turn.
    dials: Mutex<HashMap<Identity, u64>>,
    /// The number the next dial task is given.
    next_dial: Mutex<u64>,
    /// The current dial task of every provider dialled, by its
    /// identity, to be ended when the daemon stops: replaced by the
    /// next dial's, and gone with the dial's end or the task's.
    dial_tasks: Mutex<HashMap<Identity, AbortHandle>>,
    /// The closer of every provider connection held now, by the
    /// provider's identity — what [`close_provider`](Self::close_provider)
    /// tells — and how that connection was made, with the IP at its
    /// other end: what an edit of the record's `ips` judges it by.
    provider_closers: Mutex<HashMap<Identity, (watch::Sender<bool>, Source)>>,
    /// What the daemon holds open on each connected provider's
    /// connection as its record says — its announce, its accept and its
    /// listen: there from the provider's standing as connected to the
    /// connection's end. See [`Features`].
    features: Mutex<HashMap<Identity, Arc<Features>>>,
    /// Every agent's log that has been appended to or watched since
    /// the start, by agent, while the agent is on record: see [`kept`].
    logs: Mutex<HashMap<AgentId, Arc<RunLog>>>,
    /// Every tool's log that has been appended to or watched since the
    /// start, by tool, while the tool is on record.
    tool_logs: Mutex<HashMap<ToolKey, Arc<RunLog>>>,
    /// The number the next run is given: what its log's lifecycle is
    /// kept under.
    next_epoch: Mutex<u64>,
    /// One lock per agent and tool a start is made or waited on for
    /// now, taken for the length of a start: two starts of one are
    /// served one after the other, so a run is deployed once. An entry
    /// leaves once nobody holds its lock: see [`started`](Self::started).
    starting: Mutex<HashMap<Key, Arc<Mutex<()>>>>,
    /// Every agent's outbox: the messages admitted for it and not yet
    /// taken by its pump, in the order they were admitted. An entry
    /// there is a pump running, and no entry is none: see
    /// [`admit`](Self::admit).
    outboxes: Mutex<HashMap<AgentId, Vec<Waiting>>>,
    /// Every account's and configured administrator's command log being
    /// appended to or watched now; a tool's commands are in its tool
    /// log. See [`command_log`](Self::command_log).
    command_logs: Mutex<HashMap<Commander, Arc<Log>>>,
    /// The agents running now.
    agents: Mutex<HashMap<AgentId, Arc<AgentRun>>>,
    /// The tools running now, or joined: records and dependency tools.
    tools: Mutex<HashMap<ToolKey, Arc<ToolRun>>>,
    /// Every service kept since the start, run or no run, by its tool
    /// — a record whose template's mode is a service, or a dependency
    /// tool's row whose template is one: whether it is held, and its
    /// supervisor and its clock.
    services: Mutex<HashMap<ToolKey, Arc<State>>>,
    /// Every container's database scope touched since the start: its
    /// password, in memory only.
    scopes: Mutex<HashMap<Key, Arc<Scope>>>,
    /// The backend key of every container session open now, and
    /// whose it is: what a cancel is judged by.
    backends: Mutex<HashMap<(i32, i32), Key>>,
    /// Every container connection open through the database now, by
    /// a number of the daemon's, and when it was opened.
    connections: Mutex<HashMap<u64, (Key, DateTime<Utc>)>>,
    /// The next connection number.
    next_connection: Mutex<u64>,
    /// The volumes a download, an upload or a transfer of the daemon's
    /// is on now, each held by one operation at a time.
    operations: Arc<Mutex<HashSet<reference::Volume>>>,
    /// The volumes a `volumes fuse_serve` of the daemon's serves now,
    /// each with how many serves of it are open: a volume may be
    /// served to several callers at once, where its provider allows.
    fuse_serving: Mutex<HashMap<reference::Volume, usize>>,
    /// The word that a kind changed, for the lists kept open.
    changes: Changes,
    /// Each connected provider's volumes, as its listing streams them.
    volumes: Mutex<HashMap<Identity, Arc<Mirror>>>,
    /// Each connected provider's daemons list, while something
    /// watches: see [`Listings`].
    pub listings: Listings,
    /// The ring every messageable provider's listen feeds: see
    /// [`Listening`].
    pub listening: Listening,
    /// Each provider's send lane and rate window, for the daemon's own
    /// direct messages: see [`Messaging`].
    pub messaging: Messaging,
    /// The remote daemons connected to now, by the ID of the record
    /// each was opened for, each through one provider: what a connected
    /// tool's exchanges and a relayed request ride.
    daemons: Mutex<HashMap<DaemonId, Arc<Peer>>>,
    /// One lock per remote daemon's record a connect is made or waited
    /// on for now, taken for the length of the connect: two connects of
    /// one record are made one after the other, so a record holds one
    /// connection. An entry leaves once nobody holds its lock: see
    /// [`connected_to`](Self::connected_to).
    connecting: Mutex<HashMap<DaemonId, Arc<Mutex<()>>>>,
    /// The next number for a connect scope another daemon opens.
    next_connect: Mutex<u64>,
    /// Each connected provider's capacity cell, by identity: a
    /// snapshot map, swapped whole when a provider connects or
    /// disconnects, so a reader looks a cell up under no lock at any
    /// time. The cell itself is read the same way: see [`Capacity`].
    capacities: ArcSwap<HashMap<Identity, Arc<Capacity>>>,
}

impl std::fmt::Debug for Live {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Live")
    }
}

/// One agent's log as it is live: the lock an append is made under,
/// so two land one after the other with the index right, and the
/// latest index as a watch, so a reader waiting on more learns of
/// every append. The sender gone — the agent deleted — ends every
/// watch.
#[derive(Debug)]
pub struct Log {
    /// Taken for the length of one append.
    pub lock: Mutex<()>,
    /// The `logs_index` of the latest item appended since the start;
    /// `0` before any. What a reader waits on.
    pub latest: watch::Sender<u64>,
}

impl Log {
    /// A log with nothing appended since the start.
    pub fn new() -> Self {
        Log { lock: Mutex::new(()), latest: watch::channel(0).0 }
    }
}

impl Default for Log {
    fn default() -> Self {
        Log::new()
    }
}

impl Live {
    /// Nothing live.
    pub fn new() -> Self {
        Live::default()
    }

    /// A closer for one client connection as `connected`: its key, and
    /// the word that it is to close, which [`disconnect`](Self::disconnect)
    /// turns `true`. The connection forgets it with
    /// [`close`](Self::close) when it ends.
    pub async fn open(&self, connected: Connected) -> (u64, watch::Receiver<bool>) {
        let key = {
            let mut next = self.next_closer.lock().await;
            *next += 1;
            *next
        };
        let (closer, closing) = watch::channel(false);
        self.closers.lock().await.entry(connected).or_default().insert(key, closer);
        (key, closing)
    }

    /// The connection of `key`, as `connected`, ended: its closer
    /// forgotten.
    pub async fn close(&self, connected: &Connected, key: u64) {
        let mut closers = self.closers.lock().await;
        if let Some(keyed) = closers.get_mut(connected) {
            keyed.remove(&key);
            if keyed.is_empty() {
                closers.remove(connected);
            }
        }
    }

    /// Tell every client connection as `connected` to close: how many
    /// there were. It does not wait for them to end — each ends its own
    /// session, its scopes and its count — so a connection may close
    /// itself.
    pub async fn disconnect(&self, connected: &Connected) -> usize {
        let closers = self.closers.lock().await;
        let Some(keyed) = closers.get(connected) else {
            return 0;
        };
        for closer in keyed.values() {
            closer.send_replace(true);
        }
        keyed.len()
    }

    /// Count a connection as the account in. The connection counts
    /// itself out with [`leave`](Self::leave) when it ends. Whether it
    /// is the first, which the caller keeps as the account's
    /// `last_connected`.
    pub async fn enter(&self, id: AccountId) -> bool {
        let first = {
            let mut connected = self.connected.lock().await;
            let count = connected.entry(id).or_insert(0);
            *count += 1;
            *count == 1
        };
        if first {
            self.changes.changed(Kind::Accounts);
        }
        first
    }

    /// Count a connection as the account out. Whether it was the last.
    pub async fn leave(&self, id: AccountId) -> bool {
        let last = {
            let mut connected = self.connected.lock().await;
            match connected.get_mut(&id) {
                Some(count) => {
                    *count -= 1;
                    if *count == 0 {
                        connected.remove(&id);
                        true
                    } else {
                        false
                    }
                }
                None => false,
            }
        };
        if last {
            self.changes.changed(Kind::Accounts);
        }
        last
    }

    /// Count a connection as the administrator in, as
    /// [`enter`](Self::enter) counts an account's.
    pub async fn enter_admin(&self, identity: &Identity) {
        let first = {
            let mut connected = self.admins_connected.lock().await;
            let count = connected.entry(identity.clone()).or_insert(0);
            *count += 1;
            *count == 1
        };
        if first {
            self.admins_last_connected.lock().await.insert(identity.clone(), Utc::now());
            self.changes.changed(Kind::Accounts);
        }
    }

    /// Count a connection as the administrator out.
    pub async fn leave_admin(&self, identity: &Identity) {
        let last = {
            let mut connected = self.admins_connected.lock().await;
            match connected.get_mut(identity) {
                Some(count) => {
                    *count -= 1;
                    if *count == 0 {
                        connected.remove(identity);
                        true
                    } else {
                        false
                    }
                }
                None => false,
            }
        };
        if last {
            self.admins_last_connected.lock().await.insert(identity.clone(), Utc::now());
            self.changes.changed(Kind::Accounts);
        }
    }

    /// When the administrator's connections last began or all ended,
    /// since the daemon started.
    pub async fn admin_last_connected(&self, identity: &Identity) -> Option<DateTime<Utc>> {
        self.admins_last_connected.lock().await.get(identity).copied()
    }

    /// Every administrator a client is connected as now.
    pub async fn admins_connected(&self) -> HashSet<Identity> {
        self.admins_connected.lock().await.keys().cloned().collect()
    }

    /// Whether a client is connected as the account now.
    pub async fn is_connected(&self, id: AccountId) -> bool {
        self.connected.lock().await.contains_key(&id)
    }

    /// Every account a client is connected as now: one snapshot, for
    /// a list that asks about each.
    pub async fn connected(&self) -> HashSet<AccountId> {
        self.connected.lock().await.keys().copied().collect()
    }

    /// Reserve the provider's slot for a connection its handshake
    /// admitted, holding its identity: the slot pending. Nothing is said
    /// of it yet: a pending slot reads as no connection. What is
    /// reserved is given back with `disconnect_provider`, on every
    /// path.
    pub async fn reserve_provider(&self, identity: Identity) {
        self.providers.lock().await.reserve(identity);
    }

    /// The provider answered its info with the daemon's version: its
    /// slot filled with the handle every request to it rides and the
    /// answer, cached for the connection's life, its [`Features`] kept
    /// with nothing held yet, and the word said that the provider is
    /// connected. The answer, shared, for the attach's own reading.
    pub async fn connect_provider(&self, identity: &Identity, handle: Handle, info: Info) -> (Arc<Info>, Arc<Capacity>) {
        let info = Arc::new(info);
        let features = Arc::new(Features::new(handle.clone(), Arc::clone(&info)));
        let standing = Standing::Connected {
            handle,
            info: Arc::clone(&info),
        };
        let cell = Arc::new(Capacity::new());
        if self.providers.lock().await.fill(identity, standing) {
            self.features.lock().await.insert(identity.clone(), features);
            self.capacities.rcu(|map| {
                let mut map = HashMap::clone(map);
                map.insert(identity.clone(), Arc::clone(&cell));
                map
            });
            self.changes.changed(Kind::Providers);
        }
        (info, cell)
    }

    /// The provider answered its info with a version that is not the
    /// daemon's: its slot filled with the handle, kept so the socket
    /// stays open and handed to nobody, and the answer, which a list
    /// reports; the word said, since the slot's standing changed.
    pub async fn different_version_provider(&self, identity: &Identity, handle: Handle, info: Info) {
        let standing = Standing::DifferentVersion {
            _handle: handle,
            info: Arc::new(info),
        };
        if self.providers.lock().await.fill(identity, standing) {
            self.changes.changed(Kind::Providers);
        }
    }

    /// The provider's connection ended: its slot given back, what was
    /// held open on it forgotten — every scope on it ended with the
    /// socket — and its
    /// capacity cell dropped from the map and nudged, so a poller asleep on its interval wakes, finds the
    /// connection gone and ends.
    pub async fn disconnect_provider(&self, identity: &Identity) {
        self.features.lock().await.remove(identity);
        if self.providers.lock().await.release(identity) {
            let cell = self.capacity_cell(identity);
            self.capacities.rcu(|map| {
                let mut map = HashMap::clone(map);
                map.remove(identity);
                map
            });
            if let Some(cell) = cell {
                cell.nudge();
            }
            self.changes.changed(Kind::Providers);
        }
        if self.volumes.lock().await.remove(identity).is_some() {
            self.changes.changed(Kind::Volumes);
        }
        if self.listings.disconnected(identity).await {
            self.changes.changed(Kind::Listings);
        }
        self.messaging.disconnected(identity).await;
    }

    /// What the daemon holds open on the provider's connection, if the
    /// daemon uses the provider now: what a
    /// [`sync`](crate::providers::sync) reads.
    pub async fn features(&self, identity: &Identity) -> Option<Arc<Features>> {
        self.features.lock().await.get(identity).cloned()
    }

    /// A fresh mirror for the provider's volumes, kept for its
    /// connection's life: what its listing's watch fills.
    pub async fn mirror_provider(&self, identity: Identity) -> Arc<Mirror> {
        let mirror = Arc::new(Mirror::new());
        self.volumes.lock().await.insert(identity, Arc::clone(&mirror));
        mirror
    }

    /// The provider's mirror, if it is connected.
    pub async fn mirror(&self, identity: &Identity) -> Option<Arc<Mirror>> {
        self.volumes.lock().await.get(identity).cloned()
    }

    /// The kind changed: every list of it kept open reads again.
    pub fn changed(&self, kind: Kind) {
        self.changes.changed(kind);
    }

    /// Every word from now on, for a list kept open: subscribed
    /// before the records are read.
    pub fn changes(&self) -> tokio::sync::broadcast::Receiver<Kind> {
        self.changes.subscribe()
    }

    /// Whether the daemon uses the provider now: its info cached, its
    /// version the daemon's. A provider still being asked, and one of
    /// another version, are not connected.
    pub async fn is_provider_connected(&self, identity: &Identity) -> bool {
        self.provider_status(identity).await.connected()
    }

    /// Where the daemon stands with the provider, as a list reports
    /// it.
    pub async fn provider_status(&self, identity: &Identity) -> Status {
        self.providers.lock().await.status(identity)
    }

    /// Every provider the daemon uses now: one snapshot, for whoever
    /// asks them all.
    pub async fn connected_providers(&self) -> HashSet<Identity> {
        let providers = self.providers.lock().await;
        providers
            .slots
            .iter()
            .filter(|(_, slot)| matches!(slot.standing, Standing::Connected { .. }))
            .map(|(identity, _)| identity.clone())
            .collect()
    }

    /// The handle on the provider, if the daemon uses it now.
    pub async fn provider(&self, identity: &Identity) -> Option<Handle> {
        match self.providers.lock().await.slots.get(identity).map(|slot| &slot.standing) {
            Some(Standing::Connected { handle, .. }) => Some(handle.clone()),
            _ => None,
        }
    }

    /// The provider's info — its capabilities — if the daemon uses it
    /// now: what every ask of it is held within.
    pub async fn capabilities(&self, identity: &Identity) -> Option<Arc<Info>> {
        match self.providers.lock().await.slots.get(identity).map(|slot| &slot.standing) {
            Some(Standing::Connected { info, .. }) => Some(Arc::clone(info)),
            _ => None,
        }
    }

    /// The provider's capacity cell, if the daemon uses it now: under
    /// no lock, at any time.
    pub fn capacity_cell(&self, identity: &Identity) -> Option<Arc<Capacity>> {
        self.capacities.load().get(identity).cloned()
    }

    /// The provider's latest capacity answer, if the daemon uses it
    /// now and it has answered once: under no lock, at any time.
    pub fn capacity(&self, identity: &Identity) -> Option<Arc<CapacityFrame>> {
        self.capacity_cell(identity).and_then(|cell| cell.latest())
    }

    /// Something changed the provider's capacity: its poller asks
    /// again, now. Nothing waits for the answer; a provider not
    /// connected is nothing to nudge.
    pub fn nudge_capacity(&self, identity: &Identity) {
        if let Some(cell) = self.capacity_cell(identity) {
            cell.nudge();
        }
    }


    /// A dial of `identity` begins: its number, now the provider's
    /// current dial, so that any earlier task for it ends at its next
    /// turn and one provider is dialled by one task. The earlier task's
    /// handle is let go with it: it ends itself, never torn mid-serve.
    pub async fn begin_dial(&self, identity: Identity) -> u64 {
        let number = {
            let mut next = self.next_dial.lock().await;
            *next += 1;
            *next
        };
        let mut dials = self.dials.lock().await;
        self.dial_tasks.lock().await.remove(&identity);
        dials.insert(identity, number);
        number
    }

    /// The task of dial `number` started, for the daemon's stop to end:
    /// kept while that dial is `identity`'s current one, and let go at
    /// once when it is not — a newer dial began, or this one ended
    /// already.
    pub async fn dial_task(&self, identity: Identity, number: u64, task: AbortHandle) {
        let dials = self.dials.lock().await;
        if dials.get(&identity) == Some(&number) {
            self.dial_tasks.lock().await.insert(identity, task);
        }
    }

    /// The task of dial `number` is ending: the dial, and its task's
    /// handle, forgotten when it is still `identity`'s current one.
    pub async fn dial_ended(&self, identity: &Identity, number: u64) {
        let mut dials = self.dials.lock().await;
        if dials.get(identity) == Some(&number) {
            dials.remove(identity);
            self.dial_tasks.lock().await.remove(identity);
        }
    }

    /// Whether dial `number` is still `identity`'s current one.
    pub async fn current_dial(&self, identity: &Identity, number: u64) -> bool {
        self.dials.lock().await.get(identity) == Some(&number)
    }

    /// Whether `identity` has a current dial.
    pub async fn dialling(&self, identity: &Identity) -> bool {
        self.dials.lock().await.contains_key(identity)
    }

    /// `identity` is dialled no more: its task ends at its next turn,
    /// and its handle is let go.
    pub async fn end_dial(&self, identity: &Identity) {
        let mut dials = self.dials.lock().await;
        dials.remove(identity);
        self.dial_tasks.lock().await.remove(identity);
    }

    /// End every dial task: the daemon is stopping, and nothing reads
    /// the registry after.
    pub async fn stop_dials(&self) {
        let mut dials = self.dials.lock().await;
        dials.clear();
        for (_, dial) in self.dial_tasks.lock().await.drain() {
            dial.abort();
        }
    }

    /// A closer for the connection to the provider `identity`, made as
    /// `source` says: what its serving waits on beside the socket.
    pub async fn open_provider(&self, identity: Identity, source: Source) -> watch::Receiver<bool> {
        let (closer, closed) = watch::channel(false);
        self.provider_closers.lock().await.insert(identity, (closer, source));
        closed
    }

    /// How the connection to the provider `identity` was made, and the
    /// IP at its other end, if one is held — whatever the provider's
    /// standing on it.
    pub async fn provider_source(&self, identity: &Identity) -> Option<Source> {
        self.provider_closers.lock().await.get(identity).map(|(_, source)| *source)
    }

    /// The connection to `identity` is over: its closer forgotten.
    pub async fn forget_provider(&self, identity: &Identity) {
        self.provider_closers.lock().await.remove(identity);
    }

    /// Close the connection to the provider `identity`, whichever end
    /// dialled, if one is held; it ends as the socket ending does.
    pub async fn close_provider(&self, identity: &Identity) {
        if let Some((closer, _)) = self.provider_closers.lock().await.get(identity) {
            closer.send_replace(true);
        }
    }

    /// The agent's log, live: the one every append and every watch of
    /// that log goes through, kept as [`kept`] states. A deleted
    /// agent's is a log that keeps nothing, made fresh and never kept.
    pub async fn log(&self, store: &Store, id: AgentId) -> Arc<RunLog> {
        kept(&self.logs, store, id, Key::Agent(id)).await
    }

    /// The agent is deleted: its log forgotten — nothing kept on it
    /// again, by any writer that still holds it — and let go, which
    /// ends every watch on it once the last append has let go; its
    /// start lock let go once nobody holds it.
    pub async fn end_log(&self, id: AgentId) {
        let log = self.logs.lock().await.remove(&id);
        if let Some(log) = log {
            crate::containers::lifecycle::forget(log).await;
        }
        self.started(Key::Agent(id)).await;
    }

    /// The tool's log, live: the one every append and every watch of
    /// that log goes through, kept as [`kept`] states. A deleted tool's
    /// is a log that keeps nothing, made fresh and never kept.
    pub async fn tool_log(&self, store: &Store, key: ToolKey) -> Arc<RunLog> {
        kept(&self.tool_logs, store, key, Key::Tool(key)).await
    }

    /// The tool is deleted: its log forgotten and let go, as
    /// [`end_log`](Self::end_log) does an agent's, and the tool marked
    /// working no more.
    pub async fn end_tool_log(&self, key: ToolKey) {
        let log = self.tool_logs.lock().await.remove(&key);
        if let Some(log) = log {
            crate::containers::lifecycle::forget(log).await;
        }
        self.busy_tools.lock().await.remove(&key);
        self.started(Key::Tool(key)).await;
    }

    /// A fresh number for a run.
    pub async fn epoch(&self) -> u64 {
        let mut next = self.next_epoch.lock().await;
        *next += 1;
        *next
    }

    /// The lock a start of `key` is made under: see `starting`. The
    /// caller lets it go, then calls [`started`](Self::started).
    pub async fn starting(&self, key: Key) -> Arc<Mutex<()>> {
        Arc::clone(self.starting.lock().await.entry(key).or_default())
    }

    /// A start of `key` let its lock go: the entry forgotten when nobody
    /// holds it any more. Every clone is made under the map's lock, so
    /// one found unshared here stays unshared.
    pub async fn started(&self, key: Key) {
        let mut starting = self.starting.lock().await;
        if starting.get(&key).is_some_and(|lock| Arc::strong_count(lock) == 1) {
            starting.remove(&key);
        }
    }

    /// A message admitted to the agent's outbox, behind every message
    /// admitted before it. The rules, which every one of the outbox's
    /// four methods keeps under the one map lock, so that an entry
    /// there is exactly a pump running:
    ///
    /// 1. **The first makes the entry.** A message admitted where there
    ///    is no entry makes one, and `true` is the word that it did:
    ///    the caller starts the agent's pump, at once and before it
    ///    waits on anything —
    ///    [`message::admit`](crate::containers::message::admit). Every
    ///    other is pushed behind what waits, and `false`.
    /// 2. **Only the pump takes.** [`take_waiting`](Self::take_waiting)
    ///    hands the pump everything waiting, in order, and leaves the
    ///    entry, empty: the pump is still running.
    /// 3. **Only the pump ends it.** [`waiting`](Self::waiting) is the
    ///    pump's look before each batch: an entry with nothing waiting
    ///    is removed in that look, and the pump ends on its word — so a
    ///    message admitted before the look is seen by it, and one
    ///    admitted after finds no entry and starts the next pump.
    /// 4. **A pump that died is forgotten whole.**
    ///    [`forget_outbox`](Self::forget_outbox) removes the entry with
    ///    whatever waits in it, each dropped unanswered, which its
    ///    handler reports as an error; the next message admitted
    ///    starts a pump afresh.
    pub async fn admit(&self, id: AgentId, waiting: Waiting) -> bool {
        let mut outboxes = self.outboxes.lock().await;
        match outboxes.get_mut(&id) {
            Some(outbox) => {
                outbox.push(waiting);
                false
            }
            None => {
                outboxes.insert(id, vec![waiting]);
                true
            }
        }
    }

    /// Rule 3 of [`admit`](Self::admit), the pump's look: the agent's
    /// record as the latest message waiting carries it, while anything
    /// waits; else the entry removed, and `None`, on which the pump
    /// ends.
    pub async fn waiting(&self, id: AgentId) -> Option<Agent> {
        let mut outboxes = self.outboxes.lock().await;
        match outboxes.get(&id).and_then(|outbox| outbox.last()) {
            Some(waiting) => Some(waiting.agent.clone()),
            None => {
                outboxes.remove(&id);
                None
            }
        }
    }

    /// Rule 2 of [`admit`](Self::admit): everything waiting for the
    /// agent, in the order it was admitted, taken by its pump; the
    /// entry stays.
    pub async fn take_waiting(&self, id: AgentId) -> Vec<Waiting> {
        self.outboxes.lock().await.get_mut(&id).map(std::mem::take).unwrap_or_default()
    }

    /// Rule 4 of [`admit`](Self::admit): the agent's pump died, and its
    /// entry is forgotten with whatever waits in it.
    pub async fn forget_outbox(&self, id: AgentId) {
        self.outboxes.lock().await.remove(&id);
    }

    /// A requester's command log, live: the one every append and every
    /// watch of that log goes through. The rules, so the map holds only
    /// what is in use:
    ///
    /// 1. **Made for its first user.** An entry is made by the first
    ///    append or watch that asks while there is none; every caller
    ///    is an account the store has, or an administrator the
    ///    configuration names — a watch of any other identity's takes
    ///    a [`Log`] of its own, never kept.
    /// 2. **Gone with its last.** Every user calls
    ///    [`release_command_log`](Self::release_command_log) once it
    ///    has let go of the log and of its watch, and the entry leaves
    ///    when nothing holds or watches it. Its latest index is then
    ///    the latest since the entry was made, which is all a watch
    ///    reads: what is appended after it subscribed.
    /// 3. **Gone with its account,** at once: the sender's end ends
    ///    every watch on it once the last append has let go.
    pub async fn command_log(&self, commander: Commander) -> Arc<Log> {
        let mut logs = self.command_logs.lock().await;
        Arc::clone(logs.entry(commander).or_insert_with(|| Arc::new(Log::new())))
    }

    /// Rule 2 of [`command_log`](Self::command_log): the requester's
    /// command log forgotten when nothing holds or watches it now.
    pub async fn release_command_log(&self, commander: Commander) {
        let mut logs = self.command_logs.lock().await;
        if logs.get(&commander).is_some_and(|log| Arc::strong_count(log) == 1 && log.latest.receiver_count() == 0) {
            logs.remove(&commander);
        }
    }

    /// Rule 3 of [`command_log`](Self::command_log): the account is
    /// deleted, and its command log forgotten.
    pub async fn end_command_log(&self, commander: Commander) {
        self.command_logs.lock().await.remove(&commander);
    }

    /// The agent's run, if it is up.
    pub async fn agent_run(&self, id: AgentId) -> Option<Arc<AgentRun>> {
        self.agents.lock().await.get(&id).cloned()
    }

    /// The agent is running.
    pub async fn insert_agent(&self, run: Arc<AgentRun>) {
        self.agents.lock().await.insert(run.id, run);
    }

    /// The agent's run `run` is over: forgotten as live, once, and only
    /// while it is the run live under its id — a late end of an older
    /// run never evicts a newer one. Whether it was.
    pub async fn remove_agent_if(&self, run: &Arc<AgentRun>) -> bool {
        let mut agents = self.agents.lock().await;
        match agents.get(&run.id) {
            Some(live) if Arc::ptr_eq(live, run) => agents.remove(&run.id).is_some(),
            _ => false,
        }
    }

    /// Whether `run` is the agent's run live now.
    pub async fn is_agent_run(&self, run: &Arc<AgentRun>) -> bool {
        self.agents.lock().await.get(&run.id).is_some_and(|live| Arc::ptr_eq(live, run))
    }

    /// Every agent running now.
    pub async fn agent_runs(&self) -> Vec<Arc<AgentRun>> {
        self.agents.lock().await.values().cloned().collect()
    }

    /// The agents active now — a loop running, or an exchange in
    /// flight: one snapshot for a list.
    pub async fn active_agents(&self) -> HashSet<AgentId> {
        self.agents
            .lock()
            .await
            .values()
            .filter(|run| run.is_active())
            .map(|run| run.id)
            .collect()
    }

    /// Every deployed agent's state now: one snapshot, for a list. An
    /// agent not among them is not deployed.
    pub async fn agent_states(&self) -> HashMap<AgentId, AgentState> {
        self.agents
            .lock()
            .await
            .values()
            .map(|run| (run.id, crate::serve::agents::state_of(Some(run))))
            .collect()
    }

    /// Mark the tool working, or not: a call or a read in flight.
    pub async fn set_busy(&self, key: ToolKey, busy: bool) {
        let mut tools = self.busy_tools.lock().await;
        if busy {
            tools.insert(key);
        } else {
            tools.remove(&key);
        }
    }

    /// Whether the tool is working now.
    pub async fn is_busy(&self, key: ToolKey) -> bool {
        self.busy_tools.lock().await.contains(&key)
    }

    /// Every tool working now: one snapshot, for a list.
    pub async fn busy_tools(&self) -> HashSet<ToolKey> {
        self.busy_tools.lock().await.clone()
    }

    /// The tool's run, if it is up.
    pub async fn tool_run(&self, key: ToolKey) -> Option<Arc<ToolRun>> {
        self.tools.lock().await.get(&key).cloned()
    }

    /// The tool is running, or joined.
    pub async fn insert_tool(&self, run: Arc<ToolRun>) {
        self.tools.lock().await.insert(run.id, run);
    }

    /// The tool's run `run` is over: forgotten as live, once, and only
    /// while it is the run live under its key, as
    /// [`remove_agent_if`](Self::remove_agent_if). Whether it was.
    pub async fn remove_tool_if(&self, run: &Arc<ToolRun>) -> bool {
        let mut tools = self.tools.lock().await;
        match tools.get(&run.id) {
            Some(live) if Arc::ptr_eq(live, run) => tools.remove(&run.id).is_some(),
            _ => false,
        }
    }

    /// Whether `run` is the tool's run live now.
    pub async fn is_tool_run(&self, run: &Arc<ToolRun>) -> bool {
        self.tools.lock().await.get(&run.id).is_some_and(|live| Arc::ptr_eq(live, run))
    }

    /// Every tool running now, records and dependency tools.
    pub async fn tool_runs(&self) -> Vec<Arc<ToolRun>> {
        self.tools.lock().await.values().cloned().collect()
    }

    /// The tools active now, each with its container's id when it has
    /// one: one snapshot for a list.
    pub async fn active_tools(&self) -> HashMap<ToolKey, Option<String>> {
        self.tools
            .lock()
            .await
            .values()
            .map(|run| (run.id, run.container.clone()))
            .collect()
    }

    /// The service's state, if it is kept.
    pub async fn service_state(&self, key: ToolKey) -> Option<Arc<State>> {
        self.services.lock().await.get(&key).cloned()
    }

    /// Keep `state` for the service, unless one is kept already: the
    /// one kept, and whether it is the one handed in.
    pub async fn keep_service_state(&self, key: ToolKey, state: Arc<State>) -> (Arc<State>, bool) {
        let mut services = self.services.lock().await;
        if let Some(kept) = services.get(&key) {
            return (Arc::clone(kept), false);
        }
        services.insert(key, Arc::clone(&state));
        (state, true)
    }

    /// The service is retired: its state, once.
    pub async fn remove_service_state(&self, key: ToolKey) -> Option<Arc<State>> {
        self.services.lock().await.remove(&key)
    }

    /// Every service's state.
    pub async fn service_states(&self) -> Vec<Arc<State>> {
        self.services.lock().await.values().cloned().collect()
    }

    /// The container's database scope, live: the one there is, or one
    /// made now for the role `role` names, with a fresh password.
    pub async fn scope(&self, key: Key, role: impl FnOnce() -> String) -> Arc<Scope> {
        let mut scopes = self.scopes.lock().await;
        Arc::clone(scopes.entry(key).or_insert_with(|| Arc::new(Scope::new(role()))))
    }

    /// The container is deleted: its scope is forgotten.
    pub async fn forget_scope(&self, key: Key) {
        self.scopes.lock().await.remove(&key);
    }

    /// A container session was given the backend key.
    pub async fn register_backend(&self, backend: (i32, i32), key: Key) {
        self.backends.lock().await.insert(backend, key);
    }

    /// The session is over.
    pub async fn forget_backend(&self, backend: (i32, i32)) {
        self.backends.lock().await.remove(&backend);
    }

    /// Whose session the backend key is, if any's.
    pub async fn backend_owner(&self, backend: (i32, i32)) -> Option<Key> {
        self.backends.lock().await.get(&backend).copied()
    }

    /// A container connection is open through the database: its
    /// number, to close it by.
    pub async fn open_connection(&self, key: Key) -> u64 {
        let id = {
            let mut next = self.next_connection.lock().await;
            *next += 1;
            *next
        };
        self.connections.lock().await.insert(id, (key, Utc::now()));
        self.changes.changed(Kind::Postgres);
        id
    }

    /// The connection is closed.
    pub async fn close_connection(&self, id: u64) {
        if self.connections.lock().await.remove(&id).is_some() {
            self.changes.changed(Kind::Postgres);
        }
    }

    /// Every container connection open now, oldest opened first, each
    /// by the daemon's own number for it.
    pub async fn connections(&self) -> Vec<(u64, Key, DateTime<Utc>)> {
        let mut all: Vec<(u64, Key, DateTime<Utc>)> = self
            .connections
            .lock()
            .await
            .iter()
            .map(|(id, (key, opened))| (*id, *key, *opened))
            .collect();
        all.sort_by_key(|(id, _, opened)| (*opened, *id));
        all
    }

    /// Whether a running container has the volume: one whose record
    /// names it in its mounts.
    pub async fn volume_in_run(&self, volume: &reference::Volume) -> bool {
        if self.agents.lock().await.values().any(|run| run.volumes.contains(volume)) {
            return true;
        }
        self.tools.lock().await.values().any(|run| run.volumes.contains(volume))
    }

    /// Whether an operation of the daemon's is on the volume now.
    pub async fn volume_operating(&self, volume: &reference::Volume) -> bool {
        self.operations.lock().await.contains(volume)
    }

    /// Whether a `volumes fuse_serve` of the daemon's serves the volume
    /// now, to anyone.
    pub async fn volume_fuse_served(&self, volume: &reference::Volume) -> bool {
        self.fuse_serving.lock().await.contains_key(volume)
    }

    /// One more serve of the volume is open: entered at the serve's
    /// `Serving`, and left, by [`fuse_serve_leave`](Self::fuse_serve_leave),
    /// at its scope's end — every enter matched by exactly one leave.
    pub async fn fuse_serve_enter(&self, volume: &reference::Volume) {
        *self.fuse_serving.lock().await.entry(volume.clone()).or_insert(0) += 1;
    }

    /// One serve of the volume is over: the volume is served no more
    /// once the last has left.
    pub async fn fuse_serve_leave(&self, volume: &reference::Volume) {
        let mut serving = self.fuse_serving.lock().await;
        if let Some(count) = serving.get_mut(volume) {
            *count -= 1;
            if *count == 0 {
                serving.remove(volume);
            }
        }
    }

    /// Whether the volume is held: a running container has it, an
    /// operation is on it, or a FUSE serve of the daemon's serves it.
    pub async fn volume_held(&self, volume: &reference::Volume) -> bool {
        self.volume_operating(volume).await || self.volume_in_run(volume).await || self.volume_fuse_served(volume).await
    }

    /// Take the volume for one operation: `None` when it is held, and
    /// nothing taken. What is taken is the [`VolumeHold`], given back
    /// when it is dropped, on every path — a return, an error, a
    /// panic, the operation's task aborted.
    pub async fn take_volume(&self, volume: &reference::Volume) -> Option<VolumeHold> {
        if self.volume_in_run(volume).await || self.volume_fuse_served(volume).await {
            return None;
        }
        if !self.operations.lock().await.insert(volume.clone()) {
            return None;
        }
        Some(VolumeHold { operations: Arc::clone(&self.operations), volume: Some(volume.clone()) })
    }

    /// The connection opened for the remote daemon's record of the ID,
    /// if one is held.
    pub async fn daemon_peer(&self, id: DaemonId) -> Option<Arc<Peer>> {
        self.daemons.lock().await.get(&id).cloned()
    }

    /// A connection to a remote daemon is open: held under its record's ID,
    /// replacing one that ended, which is closed as
    /// [`remove_peer`](Self::remove_peer) closes one.
    pub async fn insert_daemon(&self, peer: Arc<Peer>) {
        let replaced = self.daemons.lock().await.insert(peer.daemon, Arc::clone(&peer));
        if let Some(replaced) = replaced
            && !Arc::ptr_eq(&replaced, &peer)
        {
            replaced.closer.send_replace(true);
        }
        self.changes.changed(Kind::Daemons);
    }

    /// The peer is forgotten — its connection ended, its record
    /// deleted: its closer told, so its watcher lets the connection
    /// go, and it is no longer held, unless a newer one has taken its
    /// place.
    pub async fn remove_peer(&self, peer: &Arc<Peer>) {
        peer.closer.send_replace(true);
        let removed = {
            let mut daemons = self.daemons.lock().await;
            match daemons.get(&peer.daemon) {
                Some(held) if Arc::ptr_eq(held, peer) => daemons.remove(&peer.daemon).is_some(),
                _ => false,
            }
        };
        if removed {
            self.changes.changed(Kind::Daemons);
        }
    }

    /// Whether this daemon holds a connection for the remote daemon's
    /// record of the ID now, one that has not ended.
    pub async fn is_daemon_connected(&self, id: DaemonId) -> bool {
        self.daemons.lock().await.get(&id).is_some_and(|peer| !peer.connected.is_ended())
    }

    /// Every remote daemon's record connected for now, by ID: one
    /// snapshot, for a list.
    pub async fn connected_daemons(&self) -> HashSet<DaemonId> {
        self.daemons
            .lock()
            .await
            .iter()
            .filter(|(_, peer)| !peer.connected.is_ended())
            .map(|(id, _)| *id)
            .collect()
    }

    /// The lock a connect to the remote daemon of the record `id` is
    /// made under: see `connecting`. The caller lets it go, then calls
    /// [`connected_to`](Self::connected_to).
    pub async fn connecting(&self, id: DaemonId) -> Arc<Mutex<()>> {
        Arc::clone(self.connecting.lock().await.entry(id).or_default())
    }

    /// A connect to the record `id` let its lock go: the entry forgotten
    /// when nobody holds it any more, as [`started`](Self::started)
    /// forgets a start's.
    pub async fn connected_to(&self, id: DaemonId) {
        let mut connecting = self.connecting.lock().await;
        if connecting.get(&id).is_some_and(|lock| Arc::strong_count(lock) == 1) {
            connecting.remove(&id);
        }
    }

    /// A number for a connect scope another daemon is opening, never
    /// given twice this daemon life.
    pub async fn mint_connection(&self) -> ConnectionId {
        let mut next = self.next_connect.lock().await;
        *next += 1;
        ConnectionId(*next)
    }
}

/// One operation's hold on a volume, from
/// [`take_volume`](Live::take_volume): the volume given back when it is
/// dropped. A drop cannot wait on the lock, so the giving back is a task
/// of its own.
#[derive(Debug)]
pub struct VolumeHold {
    /// The volumes held by operations: the live set's.
    operations: Arc<Mutex<HashSet<reference::Volume>>>,
    /// The volume held, until the drop gives it back.
    volume: Option<reference::Volume>,
}

impl VolumeHold {
    /// The operation is over: the volume given back now, waited for, so
    /// the next operation on it finds it free. A hold dropped instead
    /// gives it back a moment later.
    pub async fn release(mut self) {
        // The lock first: a release cancelled while it waits leaves the
        // volume to the drop.
        let mut operations = self.operations.lock().await;
        if let Some(volume) = self.volume.take() {
            operations.remove(&volume);
        }
    }
}

impl Drop for VolumeHold {
    fn drop(&mut self) {
        let Some(volume) = self.volume.take() else {
            return;
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let operations = Arc::clone(&self.operations);
            runtime.spawn(async move {
                operations.lock().await.remove(&volume);
            });
        }
    }
}

/// A run's log out of `logs`, by `key` — `record` naming it as a
/// record. The rules, so a log is kept only while its record is:
///
/// 1. **A kept log is the one.** One in the map is handed back as it is.
/// 2. **A new one is kept first, then its record looked for.** None in
///    the map, a fresh one is kept, and only then is the store asked
///    whether the record is there. A record on record — or a store
///    that will not say — keeps it.
/// 3. **A record gone forgets it.** No record, and the log kept by
///    rule 2 is taken back out, if it is still the one kept, and
///    forgotten, so a writer that took it meanwhile keeps nothing on
///    it; the caller gets a log that keeps nothing, made fresh and
///    never kept. A deletion commits before its `end_log`, so either
///    that `end_log` finds the log rule 2 kept, or the store asked
///    after it is kept finds no record: none outlives its record.
async fn kept<K: std::hash::Hash + Eq + Copy>(logs: &Mutex<HashMap<K, Arc<RunLog>>>, store: &Store, key: K, record: Key) -> Arc<RunLog> {
    // Rules 1 and 2.
    let log = {
        let mut logs = logs.lock().await;
        if let Some(log) = logs.get(&key) {
            return Arc::clone(log);
        }
        let log = Arc::new(RunLog::new());
        logs.insert(key, Arc::clone(&log));
        log
    };
    if on_record(store, record).await {
        return log;
    }
    // Rule 3.
    {
        let mut logs = logs.lock().await;
        if logs.get(&key).is_some_and(|kept| Arc::ptr_eq(kept, &log)) {
            logs.remove(&key);
        }
    }
    crate::containers::lifecycle::forget(log).await;
    Arc::new(RunLog::forgotten())
}

/// Whether the record `key` names is on record; `true` when the store
/// will not say, so a store that fails forgets nothing.
async fn on_record(store: &Store, key: Key) -> bool {
    let Ok(mut conn) = store.acquire().await else {
        return true;
    };
    let found = match key {
        Key::Agent(id) => store::agents::by_id(&mut conn, id, false).await.map(|row| row.is_some()),
        Key::Tool(ToolKey::Record(id)) => store::tools::by_id(&mut conn, id, false).await.map(|row| row.is_some()),
        Key::Tool(ToolKey::Connected(id)) => store::connected_standalone_tools::by_id(&mut conn, id, false).await.map(|row| row.is_some()),
        Key::Tool(ToolKey::Dependency(id)) => store::dependency_tools::by_id(&mut conn, id, false).await.map(|row| row.is_some()),
    };
    found.unwrap_or(true)
}

/// Whom a client connection is connected as, for its closer: an account,
/// or an administrator with none.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Connected {
    /// An account.
    Account(AccountId),
    /// An administrator the configuration names, by identity.
    Admin(Identity),
}
