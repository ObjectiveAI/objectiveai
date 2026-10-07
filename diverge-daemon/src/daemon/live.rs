//! What is live: nobody's record, gone at restart.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::client::handle::Handle;
use tokio::sync::{Mutex, Notify, watch};
use tokio::task::AbortHandle;

use crate::containers::{AgentRun, ToolRun};
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
    /// The providers connected now, by identity, and the caller's
    /// handle on each: what every request to a provider rides.
    providers: Mutex<HashMap<Identity, Handle>>,
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
        *self.connected.lock().await.entry(id).or_insert(0) += 1;
    }

    /// Count a connection as the account out.
    pub async fn leave(&self, id: AccountId) {
        let mut connected = self.connected.lock().await;
        if let Some(count) = connected.get_mut(&id) {
            *count -= 1;
            if *count == 0 {
                connected.remove(&id);
            }
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

    /// A provider is connected, and this is the handle on it. A
    /// second connection under one identity replaces the first's
    /// handle, which is the newer one being the one that lives.
    pub async fn connect_provider(&self, identity: Identity, handle: Handle) {
        self.providers.lock().await.insert(identity, handle);
    }

    /// The provider's connection ended.
    pub async fn disconnect_provider(&self, identity: &Identity) {
        self.providers.lock().await.remove(identity);
    }

    /// Whether the daemon holds a connection to the provider now.
    pub async fn is_provider_connected(&self, identity: &Identity) -> bool {
        self.providers.lock().await.contains_key(identity)
    }

    /// Every provider connected now: one snapshot, for a list.
    pub async fn connected_providers(&self) -> HashSet<Identity> {
        self.providers.lock().await.keys().cloned().collect()
    }

    /// The handle on the provider, if it is connected.
    pub async fn provider(&self, identity: &Identity) -> Option<Handle> {
        self.providers.lock().await.get(identity).cloned()
    }

    /// Keep the dial task for the address, ending any earlier one for
    /// the same address first, so one address is dialled by one task.
    pub async fn start_dial(&self, address: String, dial: AbortHandle) {
        if let Some(earlier) = self.dials.lock().await.insert(address, dial) {
            earlier.abort();
        }
    }

    /// End the dial task for the address, and the connection it holds
    /// with it — and take the handle out, since a task ended mid-way
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
}
