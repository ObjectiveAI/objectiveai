//! What is live: nobody's record, gone at restart.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::client::handle::Handle;
use tokio::sync::{Mutex, Notify, watch};
use tokio::task::AbortHandle;

use chrono::{DateTime, Utc};

use diverge_sdk::daemon::reference;

use super::{Changes, Kind, Peers};
use crate::volumes::Mirror;
use crate::containers::{AgentRun, Key, ToolRun};
use crate::database::Scope;
use crate::store::{AccountId, AgentId, ToolId};

/// The live state of the daemon: which accounts have a client
/// connected as them, and how many; which providers the daemon holds
/// a connection to, and the handle it speaks to each on; and the dial
/// tasks keeping the outgoing ones connected; every agent's log as
/// it is appended and watched; which agents and tools run, and what
/// is live for each; the deployer agents' queues; and the word that a
/// dependency's position was answered. Behind async mutexes, held for
/// a lookup and never across anything that waits on the outside.
#[derive(Default)]
pub struct Live {
    /// Connections per account, for the accounts with any.
    connected: Mutex<HashMap<AccountId, usize>>,
    /// The providers connected now, one slot per identity and one
    /// connection per credential, and the caller's handle on each:
    /// what every request to a provider rides. See [`Peers`].
    providers: Mutex<Peers>,
    /// The dial task of every outgoing provider, by address, to be
    /// ended when the provider is forgotten or the daemon stops.
    dials: Mutex<HashMap<String, AbortHandle>>,
    /// Every agent's log that has been appended to or watched since
    /// the start, by agent.
    logs: Mutex<HashMap<AgentId, Arc<Log>>>,
    /// The agents running now.
    agents: Mutex<HashMap<AgentId, Arc<AgentRun>>>,
    /// The tools running now, or joined.
    tools: Mutex<HashMap<ToolId, Arc<ToolRun>>>,
    /// One queue per deployer agent: the lock a dependency holds for
    /// its turn, so the deployer handles one at a time.
    deployers: Mutex<HashMap<AgentId, Arc<Mutex<()>>>>,
    /// A route was set or a tool attached: every dependency waiting
    /// on its position looks again.
    pub answered: Notify,
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

    /// Whether a client is connected as the account now.
    pub async fn is_connected(&self, id: AccountId) -> bool {
        self.connected.lock().await.contains_key(&id)
    }

    /// Every account a client is connected as now: one snapshot, for
    /// a list that asks about each.
    pub async fn connected(&self) -> HashSet<AccountId> {
        self.connected.lock().await.keys().copied().collect()
    }

    /// Take the provider's slot for a connection being admitted,
    /// holding `credential` — the hash of the key an incoming provider
    /// presented — with it: `None` when a connection holds the
    /// identity, or one holds the credential, already, and the
    /// newcomer is refused; else the slot's eviction word. What is
    /// taken is given back with `disconnect_provider`, on every path.
    pub async fn take_provider(&self, identity: Identity, credential: Option<String>) -> Option<Arc<Notify>> {
        let kind = Kind::of_provider(&identity);
        let taken = self.providers.lock().await.take(identity, credential);
        if taken.is_some() {
            self.changes.changed(kind);
        }
        taken
    }

    /// The provider answered its version, and this is the handle on
    /// it: its slot filled.
    pub async fn connect_provider(&self, identity: &Identity, handle: Handle) {
        if let Some(slot) = self.providers.lock().await.slots.get_mut(identity) {
            slot.handle = Some(handle);
        }
    }

    /// The provider's connection ended: its slot given back, with the
    /// credential it held.
    pub async fn disconnect_provider(&self, identity: &Identity) {
        if self.providers.lock().await.release(identity) {
            self.changes.changed(Kind::of_provider(identity));
        }
        if self.volumes.lock().await.remove(identity).is_some() {
            self.changes.changed(Kind::Volumes);
        }
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

    /// End the provider's connection: its own task is told, closes
    /// the socket, and gives the slot back — so the slot is held
    /// until that has happened, and a connection arriving between is
    /// refused as one arriving before would be.
    pub async fn evict_provider(&self, identity: &Identity) {
        if let Some(slot) = self.providers.lock().await.slots.get(identity) {
            slot.evict.notify_one();
        }
    }

    /// Whether the daemon holds a connection to the provider now — a
    /// provider still answering its version included, since its
    /// credential is taken.
    pub async fn is_provider_connected(&self, identity: &Identity) -> bool {
        self.providers.lock().await.slots.contains_key(identity)
    }

    /// Every provider connected now: one snapshot, for a list.
    pub async fn connected_providers(&self) -> HashSet<Identity> {
        self.providers.lock().await.slots.keys().cloned().collect()
    }

    /// The handle on the provider, if it is connected and has
    /// answered its version.
    pub async fn provider(&self, identity: &Identity) -> Option<Handle> {
        self.providers.lock().await.slots.get(identity).and_then(|slot| slot.handle.clone())
    }

    /// Keep the dial task for the address, ending any earlier one for
    /// the same address first, so one address is dialled by one task.
    pub async fn start_dial(&self, address: String, dial: AbortHandle) {
        if let Some(earlier) = self.dials.lock().await.insert(address, dial) {
            earlier.abort();
        }
    }

    /// End the dial task for the address, and the connection it holds
    /// with it — and give the slot back, since a task ended mid-way
    /// does not get to. An address with no dial is nothing to end.
    pub async fn stop_dial(&self, address: &str) {
        if let Some(dial) = self.dials.lock().await.remove(address) {
            dial.abort();
        }
        self.disconnect_provider(&Identity::Outgoing {
            address: address.to_string(),
        })
        .await;
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

    /// The agents a loop runs in now: one snapshot for a list.
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
    pub async fn tool_run(&self, id: ToolId) -> Option<Arc<ToolRun>> {
        self.tools.lock().await.get(&id).cloned()
    }

    /// The tool is running, or joined.
    pub async fn insert_tool(&self, run: Arc<ToolRun>) {
        self.tools.lock().await.insert(run.id, run);
    }

    /// The tool's run is over: what was live for it, once.
    pub async fn remove_tool(&self, id: ToolId) -> Option<Arc<ToolRun>> {
        self.tools.lock().await.remove(&id)
    }

    /// Every tool running now.
    pub async fn tool_runs(&self) -> Vec<Arc<ToolRun>> {
        self.tools.lock().await.values().cloned().collect()
    }

    /// The tools active now, each with its container's id when it has
    /// one: one snapshot for a list.
    pub async fn active_tools(&self) -> HashMap<ToolId, Option<String>> {
        self.tools
            .lock()
            .await
            .values()
            .map(|run| (run.id, run.container.clone()))
            .collect()
    }

    /// Whether an active agent is served the tool now, which is what
    /// holds a route to it.
    pub async fn serving(&self, tool: ToolId) -> bool {
        for run in self.agent_runs().await {
            if run.is_active() && run.served.lock().await.entry(tool).is_some() {
                return true;
            }
        }
        false
    }

    /// The deployer agent's queue: the lock one dependency holds for
    /// its turn.
    pub async fn deployer_queue(&self, id: AgentId) -> Arc<Mutex<()>> {
        Arc::clone(self.deployers.lock().await.entry(id).or_default())
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
}
