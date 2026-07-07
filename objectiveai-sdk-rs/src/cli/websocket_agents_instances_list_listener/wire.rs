//! Wire types for the daemon `/agents/instances/list` endpoint.

/// One agent's record on the `/agents/instances/list` endpoint: identity, spawn /
/// last-active timestamps, and whether its per-instance lock is currently
/// held. Mirrors `agents instances list`'s `ResponseItem` plus the live
/// `active` flag.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_list_listener.AgentRecord")]
pub struct AgentRecord {
    /// Full hierarchy of this agent instance.
    pub agent_instance_hierarchy: String,
    /// Tag names currently bound to this AIH, newest-bound first.
    pub tags: Vec<String>,
    /// Active `message_queue` rows targeting this agent.
    pub queued: u64,
    /// Total `objectiveai.messages` rows for this agent over all time.
    pub logged: u64,
    /// Whether the agent's per-instance lock is currently held — i.e. a
    /// live process owns this agent right now.
    pub active: bool,
    /// RFC3339 timestamp of the first `objectiveai.messages` row for this
    /// agent (spawn time). `None` for an agent with no logs yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub spawned_at: Option<String>,
    /// RFC3339 timestamp the agent was last active. Meaningful only when
    /// `active` is `false` — a live agent's last-active is implicitly
    /// "now", so it is left `None` while active and stamped at the moment
    /// the lock releases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub last_active_at: Option<String>,
}

/// One event on the `/agents/instances/list` stream. The first is always a
/// [`Snapshot`](AgentEvent::Snapshot); every later one is an
/// [`Activated`](AgentEvent::Activated) or
/// [`Deactivated`](AgentEvent::Deactivated) delta. Consumers key by
/// `agent_instance_hierarchy` (the snapshot and a delta may overlap by
/// one item at connect time).
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_list_listener.AgentEvent")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// The full set of agents, sent once immediately on connect.
    #[schemars(title = "Snapshot")]
    Snapshot { agents: Vec<AgentRecord> },
    /// An agent acquired its per-instance lock (became active).
    #[schemars(title = "Activated")]
    Activated { agent: AgentRecord },
    /// An agent's record changed while it remained present — currently
    /// emitted when its bound tags change (a tag applied, moved, or
    /// removed). Carries the full refreshed record; consumers replace by
    /// `agent_instance_hierarchy`.
    #[schemars(title = "Updated")]
    Updated { agent: AgentRecord },
    /// An agent released its per-instance lock (became inactive) — on
    /// normal stream end OR holder death. `last_active_at` is the release
    /// moment.
    #[schemars(title = "Deactivated")]
    Deactivated {
        agent_instance_hierarchy: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        last_active_at: Option<String>,
    },
}
