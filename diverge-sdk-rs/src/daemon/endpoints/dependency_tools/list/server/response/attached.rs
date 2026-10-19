//! One agent that depends on a dependency tool.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::key;

/// One agent whose last registration declared the tool's template,
/// and when it last did. An instance tool has at most one, its owner;
/// a per-template tool the agents of its agent template that declare
/// it; a global tool every agent that does.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Attached {
    /// The agent, by its [`key`](crate::daemon::key): its template and
    /// its index, with its name beside when it has one.
    pub agent: key::Agent,
    /// When the agent's registration last declared the template. On
    /// the wire an RFC 3339 timestamp in UTC. Every registration that
    /// declares it moves this forward; one that leaves it out takes
    /// the agent off the tool.
    pub declared: DateTime<Utc>,
}
