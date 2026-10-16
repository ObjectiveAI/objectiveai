//! One vault key with its value.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::daemon::creator::Creator;
use crate::daemon::key;

/// One vault key as a get answers it: what a
/// [`Key`](crate::daemon::endpoints::vault::list::server::response::Key)
/// reports, with the value beside — the one place a value is sent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Entry {
    /// The name.
    pub key: String,
    /// The value, any JSON, as the last set gave it.
    pub value: Value,
    /// The agents binding it: each by its
    /// [`key`](crate::daemon::key), with its name beside when it has
    /// one. Absent when empty.
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
