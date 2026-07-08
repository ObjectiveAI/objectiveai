//! Wire types for the daemon `/agents/instances/list` endpoint.
//!
//! Deliberately minimal: each item is an AIH plus its live `active`
//! flag — nothing else. Per-agent detail (tags, spawn / last-active
//! timestamps, counters) lives on the per-agent
//! `/agents/instances/{*aih}` endpoint's `Agent` events.

/// One agent on the `/agents/instances/list` stream: its hierarchy and
/// whether its per-instance lock is currently held. That's all this
/// endpoint carries.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_list_listener.AgentStatus")]
pub struct AgentStatus {
    /// Full hierarchy of this agent instance.
    pub agent_instance_hierarchy: String,
    /// Whether the agent's per-instance lock is currently held — i.e.
    /// a live process owns this agent right now.
    pub active: bool,
}

/// One event on the `/agents/instances/list` stream. The first is
/// always a [`Snapshot`](AgentEvent::Snapshot); every later one flips
/// one AIH's `active` flag as its instance lock is acquired or
/// released (an [`Activated`](AgentEvent::Activated) for an unseen AIH
/// introduces it).
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_list_listener.AgentEvent")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// The full agent set (every known AIH with its `active` flag),
    /// sent once immediately on connect.
    #[schemars(title = "Snapshot")]
    Snapshot { agents: Vec<AgentStatus> },
    /// An agent acquired its per-instance lock (became active).
    #[schemars(title = "Activated")]
    Activated { agent_instance_hierarchy: String },
    /// An agent released its per-instance lock (became inactive) — on
    /// normal stream end OR holder death. For the release timestamp,
    /// tags, and counters, use `/agents/instances/{*aih}`.
    #[schemars(title = "Deactivated")]
    Deactivated { agent_instance_hierarchy: String },
}
