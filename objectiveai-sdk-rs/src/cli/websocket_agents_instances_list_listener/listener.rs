//! Materialized consumer of the cli daemon's `/agents/instances/list`
//! endpoint.
//!
//! [`WebSocketAgentsInstancesListListener`] is NOT a raw event stream —
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
//! Three ways to observe it:
//! - [`agents`](WebSocketAgentsInstancesListListener::agents) — async
//!   snapshot of the current set (sorted by AIH).
//! - an on-change **callback**
//!   ([`on_change`](WebSocketAgentsInstancesListListenerBuilder::on_change)),
//!   invoked with the full refreshed set on every applied change.
//! - [`subscribe`](WebSocketAgentsInstancesListListener::subscribe) —
//!   async, blocks until the next change.
//!
//! One listener = one connection: the internal pump runs until the daemon
//! socket closes; after that the view is frozen at its last state.
//! Dropping the listener aborts the pump. Reconnection (the daemon's
//! address changes across restarts) is the caller's loop — build a new
//! listener.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use super::{AgentEvent, AgentStatus};

/// The on-change callback: invoked with the full current agent set
/// (sorted by AIH) after each applied [`AgentEvent`].
pub type OnChange = Box<dyn Fn(&[AgentStatus]) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request builder rejected the URL, or opening the SSE
    /// stream failed.
    #[error("connect daemon sse: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The underlying reqwest client failed to build.
    #[error("daemon sse http client: {0}")]
    Client(#[from] reqwest::Error),
}

/// The shared inner state, held by both the listener handle and its pump
/// task.
struct Shared {
    /// `AIH → active`. A `BTreeMap` so iteration (snapshots, the
    /// callback) is sorted by AIH.
    state: Mutex<BTreeMap<String, bool>>,
    /// A monotonically-bumped change counter. Each applied event bumps it,
    /// waking every [`subscribe`](WebSocketAgentsInstancesListListener::subscribe)
    /// waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with the full set after each change.
    on_change: Option<OnChange>,
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

/// Unconnected configuration — [`WebSocketAgentsInstancesListListener::new`] +
/// [`WebSocketAgentsInstancesListListenerBuilder::signature`] +
/// [`WebSocketAgentsInstancesListListenerBuilder::connect`].
pub struct WebSocketAgentsInstancesListListenerBuilder {
    /// Full connect URL of the daemon's agents route, e.g.
    /// `http://127.0.0.1:49152/agents/instances/list`.
    url: String,
    /// Optional auth signature, sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header.
    signature: Option<String>,
    /// Optional on-change callback.
    on_change: Option<OnChange>,
}

impl WebSocketAgentsInstancesListListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header — the daemon's SSE
    /// watcher routes authenticate by header. Without it the daemon
    /// must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with the full current agent set
    /// (sorted by AIH) after every applied change. Runs on the pump
    /// task, so keep it cheap and non-blocking; for the full state on
    /// demand use
    /// [`agents`](WebSocketAgentsInstancesListListener::agents).
    pub fn on_change(
        mut self,
        callback: impl Fn(&[AgentStatus]) + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Open the SSE stream and start the pump. The
    /// returned [`WebSocketAgentsInstancesListListener`] immediately begins
    /// folding events (the first is the endpoint's connect-time snapshot).
    pub async fn connect(self) -> Result<WebSocketAgentsInstancesListListener, Error> {
        let source = connect_sse(&self.url, self.signature.as_deref())?;

        let shared = Arc::new(Shared {
            state: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
            on_change: self.on_change,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(WebSocketAgentsInstancesListListener { shared, pump })
    }
}

/// The materialized `/agents/instances/list` view — see the module docs.
/// Construct via [`WebSocketAgentsInstancesListListener::new`]. Dropping it
/// aborts the background pump.
pub struct WebSocketAgentsInstancesListListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl WebSocketAgentsInstancesListListener {
    /// Start building a listener for the daemon's `/agents/instances/list`
    /// URL (the daemon's published base address + `/agents/instances/list`).
    pub fn new(url: impl Into<String>) -> WebSocketAgentsInstancesListListenerBuilder {
        WebSocketAgentsInstancesListListenerBuilder {
            url: url.into(),
            signature: None,
            on_change: None,
        }
    }

    /// Snapshot the current agent set, sorted by AIH.
    pub async fn agents(&self) -> Vec<AgentStatus> {
        Shared::statuses(&*self.shared.state.lock().await)
    }

    /// Block until the next change is applied to the state. A fresh call
    /// waits for the FIRST change after it is made, so a change that lands
    /// between a preceding [`agents`](Self::agents) read and this call is
    /// not observed by it — pair with the read in a loop, or use the
    /// [`on_change`](WebSocketAgentsInstancesListListenerBuilder::on_change)
    /// callback for guaranteed push.
    pub async fn subscribe(&self) {
        // A receiver from `subscribe` is caught up to the current version,
        // so `changed` resolves on the next bump. `Err` only if the sender
        // dropped (pump gone) — treat as "no more changes" and return.
        let mut rx = self.shared.changes.subscribe();
        let _ = rx.changed().await;
    }
}

impl Drop for WebSocketAgentsInstancesListListener {
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

/// Read frames, fold each `AgentEvent` into `shared.state`, fire the
/// callback with the refreshed set, and bump the change counter. Runs until
/// the connection closes. Parse errors and non-text frames are skipped;
/// transport errors end the pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<AgentEvent>(&message.data) {
                    Ok(event) => {
                        let snapshot = {
                            let mut state = shared.state.lock().await;
                            apply_event(&mut state, event);
                            shared.on_change.as_ref().map(|_| Shared::statuses(&state))
                        };
                        if let (Some(callback), Some(snapshot)) =
                            (&shared.on_change, &snapshot)
                        {
                            callback(snapshot);
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


/// Open the daemon's SSE watcher stream: request
/// `text/event-stream`, and stamp `X-OBJECTIVEAI-SIGNATURE` when a
/// signature is present (the daemon's watcher routes moved auth from
/// the first-frame preamble to this header).
fn connect_sse(
    url: &str,
    signature: Option<&str>,
) -> Result<reqwest_eventsource::EventSource, Error> {
    let client = reqwest::Client::builder().build()?;
    let mut request = client
        .get(url)
        .header("Accept", "text/event-stream");
    if let Some(signature) = signature {
        request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
    }
    Ok(request.eventsource()?)
}
