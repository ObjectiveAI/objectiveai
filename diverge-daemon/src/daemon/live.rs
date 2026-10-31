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
use crate::containers::{AgentRun, ConnectionId, Key, ToolKey, ToolRun};
use crate::daemons::Peer;
use crate::database::Scope;
use crate::store::{AccountId, AgentId};
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
    /// The administrators connected now, by identity, and how many
    /// connections each: they have no account to count by.
    admins_connected: Mutex<HashMap<Identity, usize>>,
    /// The providers connected now, one slot per identity, and the
    /// caller's handle on each: what every request to a provider
    /// rides. See [`Peers`].
    providers: Mutex<Peers>,
    /// The dial task of every provider the daemon dials, by address,
    /// to be ended when the daemon stops.
    dials: Mutex<HashMap<String, AbortHandle>>,
    /// The address each provider was last reached at by a dial, by
    /// the identity it answered the handshake as: what a list reports
    /// as its address, since an entry the daemon dials names an
    /// address and an issuer, never the identity that answers there.
    addresses: Mutex<HashMap<Identity, String>>,
    /// Every agent's log that has been appended to or watched since
    /// the start, by agent.
    logs: Mutex<HashMap<AgentId, Arc<Log>>>,
    /// Every tool's log that has been appended to or watched since the
    /// start, by tool.
    tool_logs: Mutex<HashMap<ToolKey, Arc<ToolLog>>>,
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

/// One tool's log as it is live: the lock an append is made under,
/// holding how many of the tool's asks are in flight — what its
/// `active` and `inactive` items mark — and the latest index as a
/// watch, as [`Log`]'s. The sender gone — the tool deleted — ends
/// every watch.
#[derive(Debug)]
pub struct ToolLog {
    /// Taken for the length of one append, or of an ask's two: the
    /// asks logged and not yet answered.
    pub lock: Mutex<u64>,
    /// The `logs_index` of the latest item appended since the start;
    /// `0` before any. What a reader waits on.
    pub latest: watch::Sender<u64>,
}

impl Live {
    /// Nothing live.
    pub fn new() -> Self {
        Live::default()
    }

    /// Count a connection as the account in. The connection counts
    /// itself out with [`leave`](Self::leave) when it ends.
    pub async fn enter(&self, id: AccountId) {
        let first = {
            let mut connected = self.connected.lock().await;
            let count = connected.entry(id).or_insert(0);
            *count += 1;
            *count == 1
        };
        if first {
            self.changes.changed(Kind::Accounts);
        }
    }

    /// Count a connection as the account out.
    pub async fn leave(&self, id: AccountId) {
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
            self.changes.changed(Kind::Accounts);
        }
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


    /// Keep the dial task for the address, ending any earlier one for
    /// the same address first, so one address is dialled by one task.
    pub async fn start_dial(&self, address: String, dial: AbortHandle) {
        if let Some(earlier) = self.dials.lock().await.insert(address, dial) {
            earlier.abort();
        }
    }

    /// A dial to `address` was answered by the provider `identity`:
    /// remembered as its address, for the daemon's life.
    pub async fn dialled(&self, identity: Identity, address: String) {
        let changed = self.addresses.lock().await.insert(identity, address.clone()).is_none_or(|earlier| earlier != address);
        if changed {
            self.changes.changed(Kind::Providers);
        }
    }

    /// The address the provider was last reached at by a dial, if a
    /// dial has reached it.
    pub async fn address(&self, identity: &Identity) -> Option<String> {
        self.addresses.lock().await.get(identity).cloned()
    }

    /// End every dial task: the daemon is stopping, and nothing reads
    /// the registry after.
    pub async fn stop_dials(&self) {
        for (_, dial) in self.dials.lock().await.drain() {
            dial.abort();
        }
    }

    /// The agent's log, live: the one every append and every watch of
    /// that log goes through.
    pub async fn log(&self, id: AgentId) -> Arc<Log> {
        let mut logs = self.logs.lock().await;
        Arc::clone(logs.entry(id).or_insert_with(|| {
            Arc::new(Log {
                lock: Mutex::new(()),
                latest: watch::channel(0).0,
            })
        }))
    }

    /// The agent is deleted: forget its log, which ends every watch on
    /// it once the last append has let go.
    pub async fn end_log(&self, id: AgentId) {
        self.logs.lock().await.remove(&id);
    }

    /// The tool's log, live: the one every append and every watch of
    /// that log goes through.
    pub async fn tool_log(&self, key: ToolKey) -> Arc<ToolLog> {
        let mut logs = self.tool_logs.lock().await;
        Arc::clone(logs.entry(key).or_insert_with(|| {
            Arc::new(ToolLog {
                lock: Mutex::new(0),
                latest: watch::channel(0).0,
            })
        }))
    }

    /// The tool is deleted: forget its log, which ends every watch on
    /// it once the last append has let go.
    pub async fn end_tool_log(&self, key: ToolKey) {
        self.tool_logs.lock().await.remove(&key);
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
