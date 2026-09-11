//! The agent, registered once for the container's life.
//!
//! The server tells the container its agent exactly once, at
//! `POST /register`, before the first loop; the agent never changes
//! after. Every run reads it from here, and a run before it is
//! refused.

use std::sync::OnceLock;

use crate::agent::Agent;

/// The agent, once registered.
static AGENT: OnceLock<Agent> = OnceLock::new();

/// Register the agent. `Err` hands it back: one is registered
/// already, and the agent never changes.
pub fn register(agent: Agent) -> Result<(), Agent> {
    AGENT.set(agent)
}

/// The registered agent, if there is one. A clone: the agent is
/// small, and every run wants its own.
pub fn registered() -> Option<Agent> {
    AGENT.get().cloned()
}
