//! One volume, as a provider lists it and the daemon knows it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;
use crate::daemon::key;
use crate::provider::endpoints::volumes::Mode;

/// One volume: the provider that holds it, and what that provider's
/// listing says of it — its name, how many bytes it reserves, when it
/// came into being, its mode — with the agents and the tools of the
/// daemon's that name it in their mounts, and its tags, which are the
/// daemon's. No creator: a volume is the provider's. How much of it is
/// used, and the hash of its content, cost the provider a walk, and
/// are a [`stat`](crate::daemon::endpoints::volumes::stat)'s.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Volume {
    /// The provider that holds it: see [`Identity`].
    pub provider: Identity,
    /// The name the provider lists it under: the whole of how it is
    /// named, with the provider.
    pub name: String,
    /// How many bytes it reserves, as the provider lists it.
    pub bytes: u64,
    /// The mode it is in: see [`Mode`].
    pub mode: Mode,
    /// When it came into being, as the provider says: its listing's
    /// seconds since the Unix epoch, as an RFC 3339 timestamp in UTC.
    pub created: DateTime<Utc>,
    /// The agents that name it in their mounts — a volume mount of the
    /// provider they are pinned to, or a FUSE mount of a file or a
    /// directory in it — running or not, in no order: see
    /// [`key::Agent`]. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<key::Agent>,
    /// The tools that name it in their mounts, the same way: see
    /// [`key::Tool`]. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<key::Tool>,
    /// The tags on it, as [`tag`](crate::daemon::endpoints::volumes::tag)
    /// put them: the daemon's, not the provider's. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
