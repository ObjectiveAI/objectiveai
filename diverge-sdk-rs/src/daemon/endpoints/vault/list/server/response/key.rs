//! One vault key, as the daemon holds it, its value apart.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::key;

/// One vault key: its name, the agents and tools binding it, its
/// tags, when it was made and by whom — and never its value, which a
/// [`get`](crate::daemon::endpoints::vault::get) alone answers. A list
/// sends them oldest created first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Key {
    /// The name.
    pub key: String,
    /// The agents binding it in their `secrets`: each by its
    /// [`key`](crate::daemon::key), with its name beside when it has
    /// one, in no order. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<key::Agent>,
    /// The standalone tools binding it, likewise. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standalone_tools: Vec<key::Tool>,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::vault::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::vault::untag) has not taken
    /// off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// When the first set made it. On the wire an RFC 3339 timestamp
    /// in UTC.
    pub created: DateTime<Utc>,
    /// Who made it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker.
    pub creator: Creator,
}
