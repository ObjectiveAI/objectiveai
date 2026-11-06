//! What is live: nobody's record, gone at restart.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use arc_swap::ArcSwap;
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::daemon::endpoints::providers::Status;
use diverge_sdk::provider::endpoints::capacity::server::response::Frame as CapacityFrame;
use diverge_sdk::provider::endpoints::info::server::response::Frame as Info;
use diverge_sdk::wire::client::handle::Handle;
use tokio::sync::{Mutex, watch};
use tokio::task::AbortHandle;

use chrono::{DateTime, Utc};

use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::images::oci::client::execute;

use super::{Capacity, Changes, Kind, Peers, Standing};
use crate::containers::services::State;
use crate::commands::Commander;
use crate::containers::lifecycle::RunLog;
use crate::containers::{AgentRun, ConnectionId, Key, ToolKey, ToolRun};
use crate::daemons::Peer;
use crate::judge::AgentState;
use crate::database::Scope;
use crate::store::{AccountId, AgentId};
use crate::discovery::Discovery;
use crate::volumes::Mirror;

/// The live state of the daemon: which accounts have a client
/// connected as them, and how many; which providers the daemon holds
/// a connection to, the handle it speaks to each on, the info each
/// answered and the capacity each last answered; and the dial
/// tasks keeping the ones it dials connected; which remote daemons it
/// holds a connection to through them; which providers hold which image,
/// and the pull scope held on each; every agent's log as
/// it is appended and watched; which agents and tools run — a tool
/// on record, or a dependency tool — and what is
/// live for each. Behind async mutexes, held for a lookup and never
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
    /// logs mark them `active`.
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
    /// Every dial task started, to be ended when the daemon stops.
    dial_tasks: Mutex<Vec<AbortHandle>>,
    /// The closer of every provider connection held now, by the
    /// provider's identity: what [`close_provider`](Self::close_provider)
    /// tells.
    provider_closers: Mutex<HashMap<Identity, watch::Sender<bool>>>,
    /// Every agent's log that has been appended to or watched since
    /// the start, by agent.
    logs: Mutex<HashMap<AgentId, Arc<RunLog>>>,
    /// Every tool's log that has been appended to or watched since the
    /// start, by tool.
    tool_logs: Mutex<HashMap<ToolKey, Arc<RunLog>>>,
    /// The agents and tools deleted since the start, whose logs are
    /// forgotten: a writer of one gets a log that keeps nothing, and
    /// none is made again. Ids are never reused.
    forgotten: Mutex<HashSet<Key>>,
    /// The number the next run is given: what its log's lifecycle is
    /// kept under.
    next_epoch: Mutex<u64>,
    /// One lock per agent and tool, taken for the length of a start:
    /// two starts of one are served one after the other, so a run is
    /// deployed once.
    starting: Mutex<HashMap<Key, Arc<Mutex<()>>>>,
    /// Every account's and administrator's command log appended to or
    /// watched since the start; a tool's commands are in its tool log.
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
    operations: Mutex<HashSet<reference::Volume>>,
    /// The word that a kind changed, for the lists kept open.
    changes: Changes,
    /// Each connected provider's volumes, as its listing streams them.
    volumes: Mutex<HashMap<Identity, Arc<Mirror>>>,
    /// Each connected provider's discover listing, while something
    /// watches: see [`Discovery`].
    pub discovery: Discovery,
    /// The remote daemons connected to now, by the name the
    /// configuration gives each, each through one provider: what a
    /// connected tool's exchanges and a relayed request ride.
    daemons: Mutex<HashMap<String, Arc<Peer>>>,
    /// The next number for a connect scope another daemon opens.
    next_connect: Mutex<u64>,
    /// Which providers are known to hold which image, by digest: what
    /// a provider's has ask is answered from, and where a manifest or
    /// a blob is pulled from. Positives only, learned from checks; a
    /// provider's entries go with its connection, or with a pull from
    /// it that failed.
    holders: Mutex<HashMap<String, Vec<Identity>>>,
    /// The pull scope held open on each provider pulled from: opened
    /// at the first pull, kept for the connection's life.
    pulls: Mutex<HashMap<Identity, Arc<execute::ExecuteHandle>>>,
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
    /// answer, cached for the connection's life, and the word said
    /// that the provider is connected. The answer, shared, for the
    /// attach's own reading.
    pub async fn connect_provider(&self, identity: &Identity, handle: Handle, info: Info) -> (Arc<Info>, Arc<Capacity>) {
        let info = Arc::new(info);
        let standing = Standing::Connected {
            handle,
            info: Arc::clone(&info),
        };
        let cell = Arc::new(Capacity::new());
        if self.providers.lock().await.fill(identity, standing) {
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

    /// The provider's connection ended: its slot given back, and its
    /// capacity cell dropped from the map and nudged, so a poller asleep on its interval wakes, finds the
    /// connection gone and ends.
    pub async fn disconnect_provider(&self, identity: &Identity) {
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
        if self.discovery.disconnected(identity).await {
            self.changes.changed(Kind::Discover);
        }
        for holders in self.holders.lock().await.values_mut() {
            holders.retain(|holder| holder != identity);
        }
        self.pulls.lock().await.remove(identity);
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

    /// The providers known to hold the image under `digest`, in the
    /// order they were learned.
    pub async fn holders(&self, digest: &str) -> Vec<Identity> {
        self.holders.lock().await.get(digest).cloned().unwrap_or_default()
    }

    /// The provider answered that it holds the image: remembered,
    /// once.
    pub async fn add_holder(&self, digest: &str, identity: Identity) {
        let mut holders = self.holders.lock().await;
        let known = holders.entry(digest.to_string()).or_default();
        if !known.contains(&identity) {
            known.push(identity);
        }
    }

    /// The provider does not hold the image after all, or could not
    /// be pulled from: forgotten for this digest.
    pub async fn drop_holder(&self, digest: &str, identity: &Identity) {
        if let Some(known) = self.holders.lock().await.get_mut(digest) {
            known.retain(|holder| holder != identity);
        }
    }

    /// The pull scope held on the provider, if one is.
    pub async fn pull_scope(&self, identity: &Identity) -> Option<Arc<execute::ExecuteHandle>> {
        self.pulls.lock().await.get(identity).cloned()
    }

    /// Keep `scope` as the provider's pull scope, unless one is kept
    /// already: the one kept, either way, so a second opener uses the
    /// first's and lets its own go.
    pub async fn keep_pull_scope(&self, identity: Identity, scope: Arc<execute::ExecuteHandle>) -> Arc<execute::ExecuteHandle> {
        let mut pulls = self.pulls.lock().await;
        Arc::clone(pulls.entry(identity).or_insert(scope))
    }

    /// The provider's pull scope let go: a pull from it failed, and
    /// the next opens anew.
    pub async fn drop_pull_scope(&self, identity: &Identity) {
        self.pulls.lock().await.remove(identity);
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
    /// turn and one provider is dialled by one task.
    pub async fn begin_dial(&self, identity: Identity) -> u64 {
        let number = {
            let mut next = self.next_dial.lock().await;
            *next += 1;
            *next
        };
        self.dials.lock().await.insert(identity, number);
        number
    }

    /// The dial task `task` started, for the daemon's stop to end.
    pub async fn dial_task(&self, task: AbortHandle) {
        let mut tasks = self.dial_tasks.lock().await;
        tasks.retain(|task| !task.is_finished());
        tasks.push(task);
    }

    /// Whether dial `number` is still `identity`'s current one.
    pub async fn current_dial(&self, identity: &Identity, number: u64) -> bool {
        self.dials.lock().await.get(identity) == Some(&number)
    }

    /// Whether `identity` has a current dial.
    pub async fn dialling(&self, identity: &Identity) -> bool {
        self.dials.lock().await.contains_key(identity)
    }

    /// `identity` is dialled no more: its task ends at its next turn.
    pub async fn end_dial(&self, identity: &Identity) {
        self.dials.lock().await.remove(identity);
    }

    /// End every dial task: the daemon is stopping, and nothing reads
    /// the registry after.
    pub async fn stop_dials(&self) {
        self.dials.lock().await.clear();
        for dial in self.dial_tasks.lock().await.drain(..) {
            dial.abort();
        }
    }

    /// A closer for the connection to the provider `identity`: what its
    /// serving waits on beside the socket.
    pub async fn open_provider(&self, identity: Identity) -> watch::Receiver<bool> {
        let (closer, closed) = watch::channel(false);
        self.provider_closers.lock().await.insert(identity, closer);
        closed
    }

    /// The connection to `identity` is over: its closer forgotten.
    pub async fn forget_provider(&self, identity: &Identity) {
        self.provider_closers.lock().await.remove(identity);
    }

    /// Close the connection to the provider `identity`, whichever end
    /// dialled, if one is held; it ends as the socket ending does.
    pub async fn close_provider(&self, identity: &Identity) {
        if let Some(closer) = self.provider_closers.lock().await.get(identity) {
            closer.send_replace(true);
        }
    }

    /// The agent's log, live: the one every append and every watch of
    /// that log goes through. A deleted agent's is a log that keeps
    /// nothing, made fresh and never kept.
    pub async fn log(&self, id: AgentId) -> Arc<RunLog> {
        if self.forgotten.lock().await.contains(&Key::Agent(id)) {
            return Arc::new(RunLog::forgotten());
        }
        let mut logs = self.logs.lock().await;
        Arc::clone(logs.entry(id).or_insert_with(|| Arc::new(RunLog::new())))
    }

    /// The agent is deleted: its log forgotten — nothing kept on it
    /// again, by any writer that still holds it — and let go, which
    /// ends every watch on it once the last append has let go.
    pub async fn end_log(&self, id: AgentId) {
        self.forgotten.lock().await.insert(Key::Agent(id));
        let log = self.logs.lock().await.remove(&id);
        if let Some(log) = log {
            crate::containers::lifecycle::forget(log).await;
        }
    }

    /// The tool's log, live: the one every append and every watch of
    /// that log goes through. A deleted tool's is a log that keeps
    /// nothing, made fresh and never kept.
    pub async fn tool_log(&self, key: ToolKey) -> Arc<RunLog> {
        if self.forgotten.lock().await.contains(&Key::Tool(key)) {
            return Arc::new(RunLog::forgotten());
        }
        let mut logs = self.tool_logs.lock().await;
        Arc::clone(logs.entry(key).or_insert_with(|| Arc::new(RunLog::new())))
    }

    /// The tool is deleted: its log forgotten and let go, as
    /// [`end_log`](Self::end_log) does an agent's.
    pub async fn end_tool_log(&self, key: ToolKey) {
        self.forgotten.lock().await.insert(Key::Tool(key));
        let log = self.tool_logs.lock().await.remove(&key);
        if let Some(log) = log {
            crate::containers::lifecycle::forget(log).await;
        }
    }

    /// A fresh number for a run.
    pub async fn epoch(&self) -> u64 {
        let mut next = self.next_epoch.lock().await;
        *next += 1;
        *next
    }

    /// The lock a start of `key` is made under: see `starting`.
    pub async fn starting(&self, key: Key) -> Arc<Mutex<()>> {
        Arc::clone(self.starting.lock().await.entry(key).or_default())
    }

    /// A requester's command log, live: the one every append and every
    /// watch of that log goes through.
    pub async fn command_log(&self, commander: Commander) -> Arc<Log> {
        let mut logs = self.command_logs.lock().await;
        Arc::clone(logs.entry(commander).or_insert_with(|| {
            Arc::new(Log {
                lock: Mutex::new(()),
                latest: watch::channel(0).0,
            })
        }))
    }

    /// The account is deleted: forget its command log, which ends every
    /// watch on it once the last append has let go.
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

    /// The agent's run is over: what was live for it, once.
    pub async fn remove_agent(&self, id: AgentId) -> Option<Arc<AgentRun>> {
        self.agents.lock().await.remove(&id)
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

    /// The tool's run is over: what was live for it, once.
    pub async fn remove_tool(&self, key: ToolKey) -> Option<Arc<ToolRun>> {
        self.tools.lock().await.remove(&key)
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

    /// Whether the volume is held: a running container has it, or an
    /// operation is on it.
    pub async fn volume_held(&self, volume: &reference::Volume) -> bool {
        self.volume_operating(volume).await || self.volume_in_run(volume).await
    }

    /// Take the volume for one operation: `false` when it is held,
    /// and nothing taken. What is taken is given back with
    /// [`release_volume`](Self::release_volume), on every path.
    pub async fn take_volume(&self, volume: &reference::Volume) -> bool {
        if self.volume_in_run(volume).await {
            return false;
        }
        self.operations.lock().await.insert(volume.clone())
    }

    /// The operation is over.
    pub async fn release_volume(&self, volume: &reference::Volume) {
        self.operations.lock().await.remove(volume);
    }

    /// The connection to the remote daemon of the name, if one is
    /// held.
    pub async fn daemon_peer(&self, name: &str) -> Option<Arc<Peer>> {
        self.daemons.lock().await.get(name).cloned()
    }

    /// A connection to a remote daemon is open: held under its name,
    /// replacing one that ended.
    pub async fn insert_daemon(&self, peer: Arc<Peer>) {
        self.daemons.lock().await.insert(peer.daemon.clone(), peer);
        self.changes.changed(Kind::Daemons);
    }

    /// The peer's connection ended: forgotten, unless a newer one has
    /// taken its place.
    pub async fn remove_peer(&self, peer: &Arc<Peer>) {
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

    /// Whether this daemon holds a connection to the remote daemon of
    /// the name now, one that has not ended.
    pub async fn is_daemon_connected(&self, name: &str) -> bool {
        self.daemons.lock().await.get(name).is_some_and(|peer| !peer.connected.is_ended())
    }

    /// Every remote daemon connected to now, by name: one snapshot,
    /// for a list.
    pub async fn connected_daemons(&self) -> HashSet<String> {
        self.daemons
            .lock()
            .await
            .iter()
            .filter(|(_, peer)| !peer.connected.is_ended())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// A number for a connect scope another daemon is opening, never
    /// given twice this daemon life.
    pub async fn mint_connection(&self) -> ConnectionId {
        let mut next = self.next_connect.lock().await;
        *next += 1;
        ConnectionId(*next)
    }
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
