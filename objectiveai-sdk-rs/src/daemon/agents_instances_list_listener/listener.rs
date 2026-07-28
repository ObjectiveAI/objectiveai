//! Materialized consumer of the cli daemon's `/agents/instances/list`
//! endpoint.
//!
//! [`AgentsInstancesListListener`] is NOT a raw event stream —
//! it connects once, then folds every incoming [`AgentEvent`] into an
//! in-memory, self-updating map of `AIH → active` flags: a
//! [`Snapshot`](AgentEvent::Snapshot) replaces the whole set,
//! [`Activated`](AgentEvent::Activated) upserts one AIH to active
//! (introducing it if unseen), and
//! [`Deactivated`](AgentEvent::Deactivated) flips one to inactive
//! (kept in the set — the endpoint lists all known agents). Nothing
//! else rides this endpoint — per-agent detail (tags, timestamps,
//! counters) is `/agents/instances/{*aih}`'s job.
//!
//! Ways to observe it:
//! - [`agents`](AgentsInstancesListListener::agents) — async
//!   snapshot of the current set (sorted by AIH).
//! - [`subscribe`](AgentsInstancesListListener::subscribe) —
//!   async, blocks until the next change.
//! - [`changes`](AgentsInstancesListListener::changes) — the raw
//!   change-counter receiver, for race-free condition waits.
//!
//! One listener = one connection: the internal pump runs until the daemon
//! socket closes; after that the view is frozen at its last state.
//! Dropping the listener aborts the pump. Reconnection (the daemon's
//! address changes across restarts) is the caller's loop — mint a new
//! listener from the client.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::Event;
use tokio::sync::{Mutex, watch};

use super::{AgentEvent, AgentStatus};
use crate::daemon::Error;

/// The shared inner state, held by both the listener handle and its pump
/// task.
struct Shared {
    /// `AIH → active`. A `BTreeMap` so iteration (snapshots) is sorted
    /// by AIH.
    state: Mutex<BTreeMap<String, bool>>,
    /// A monotonically-bumped change counter. Each applied event bumps it,
    /// waking every [`subscribe`](AgentsInstancesListListener::subscribe)
    /// waiter.
    changes: watch::Sender<u64>,
}

impl Shared {
    fn statuses(state: &BTreeMap<String, bool>) -> Vec<AgentStatus> {
        state
            .iter()
            .map(|(agent_instance_hierarchy, active)| AgentStatus {
                agent_instance_hierarchy: agent_instance_hierarchy.clone(),
                active: *active,
            })
            .collect()
    }
}

/// The materialized `/agents/instances/list` view — see the module docs.
/// Minted by
/// [`Client::agents_instances_list_listener`](crate::daemon::Client::agents_instances_list_listener);
/// returned only once the stream has OPENED. Dropping it aborts the
/// background pump.
pub struct AgentsInstancesListListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl AgentsInstancesListListener {
    /// Open the SSE stream (awaiting the open frame) and start the
    /// pump. The listener immediately begins folding events (the first
    /// is the endpoint's connect-time snapshot).
    pub(crate) async fn connect(
        client: &crate::daemon::Client,
    ) -> Result<AgentsInstancesListListener, Error> {
        let source = client.open_sse("/agents/instances/list").await?;
        let shared = Arc::new(Shared {
            state: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(AgentsInstancesListListener { shared, pump })
    }

    /// Snapshot the current agent set, sorted by AIH.
    pub async fn agents(&self) -> Vec<AgentStatus> {
        Shared::statuses(&*self.shared.state.lock().await)
    }

    /// Block until the next change is applied to the state. A fresh call
    /// waits for the FIRST change after it is made, so a change that lands
    /// between a preceding [`agents`](Self::agents) read and this call is
    /// not observed by it — pair with the read in a loop, or hold a
    /// [`changes`](Self::changes) receiver for race-free waits.
    pub async fn subscribe(&self) {
        // A receiver from `subscribe` is caught up to the current version,
        // so `changed` resolves on the next bump. `Err` only if the sender
        // dropped (pump gone) — treat as "no more changes" and return.
        let mut rx = self.shared.changes.subscribe();
        let _ = rx.changed().await;
    }

    /// The raw change-counter receiver — for RACE-FREE condition
    /// waits: hold ONE receiver across iterations of a
    /// check-then-await loop, and an event landing between the check
    /// and the await still resolves the next `changed()`.
    pub fn changes(&self) -> watch::Receiver<u64> {
        self.shared.changes.subscribe()
    }
}

impl Drop for AgentsInstancesListListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// Fold one event into the map. `Snapshot` replaces; `Activated`
/// upserts to active (introducing an unseen AIH); `Deactivated` flips
/// to inactive in place (kept — the endpoint lists all known agents).
fn apply_event(state: &mut BTreeMap<String, bool>, event: AgentEvent) {
    match event {
        AgentEvent::Snapshot { agents } => {
            state.clear();
            for agent in agents {
                state.insert(agent.agent_instance_hierarchy, agent.active);
            }
        }
        AgentEvent::Activated {
            agent_instance_hierarchy,
        } => {
            state.insert(agent_instance_hierarchy, true);
        }
        AgentEvent::Deactivated {
            agent_instance_hierarchy,
        } => {
            state.insert(agent_instance_hierarchy, false);
        }
    }
}

/// Read frames, fold each `AgentEvent` into `shared.state`, and bump
/// the change counter. Runs until the connection closes. Parse errors
/// and non-text frames are skipped; transport errors end the pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<AgentEvent>(&message.data) {
                    Ok(event) => {
                        {
                            let mut state = shared.state.lock().await;
                            apply_event(&mut state, event);
                        }
                        shared.changes.send_modify(|version| {
                            *version = version.wrapping_add(1);
                        });
                    }
                    // Skip a frame we can't parse rather than tearing down.
                    Err(_) => continue,
                }
            }
            Err(_) => break,
        }
    }
}
