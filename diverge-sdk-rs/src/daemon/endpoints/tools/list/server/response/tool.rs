//! One tool, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Origin;

/// One tool of the caller's: what the daemon knows of it.
///
/// Everything here the daemon holds for the tool's life, so a list
/// costs no more than the tools it names. What a created tool was
/// made from beyond the image — its limits, its mounts, its
/// arguments — is the create's, and is not repeated here; what a
/// connected tool is made from is its runner's, and unknown here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// The name, as its create gave it.
    pub name: String,
    /// Where it comes from, and what the daemon knows of its
    /// container: see [`Origin`].
    pub origin: Origin,
    /// When the create or the connect made it.
    pub created: DateTime<Utc>,
    /// Whether the tool is active now: for a created tool, its
    /// container is running; for a connected one, the daemon holds a
    /// connect scope on it. Either is so while an agent it is
    /// attached to is active and the container is there.
    pub active: bool,
    /// When that last changed: the time the container started, if it
    /// runs; the time it last stopped, if it does not; absent for a
    /// tool that has never run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
    /// The names of the agents it is attached to, in the order they
    /// were attached; empty for a tool attached nowhere. The agents
    /// list reports the same attachments from the other side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
}
