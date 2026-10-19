//! One daemon, as this daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;
use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::daemons::{Account, Link};

/// One daemon of the caller's: its name, its links, its accounts by
/// name and the kind of their mode without the credential, which of
/// them this daemon holds a connection as now, when it was added and
/// by whom, and its tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Daemon {
    /// The name, as the add gave it: the daemon's identity here.
    pub name: String,
    /// The providers it is reached through, each with the identity it
    /// is known by there: see [`Link`]. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// The accounts this daemon holds on it, in the order they were
    /// added: see [`Account`]. Absent when empty, and then nothing can
    /// act as this daemon there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<Account>,
    /// The accounts this daemon holds a connection as now, by name,
    /// each through one of the links. Absent when none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connected: Vec<String>,
    /// When the add made it. On the wire an RFC 3339 timestamp in UTC.
    pub created: DateTime<Utc>,
    /// Who added it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker.
    pub creator: Creator,
    /// The tags on it, as
    /// [`tag`](crate::daemon::endpoints::providers::daemons::tag) put
    /// them. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Keyed by the name.
impl Keyed for Daemon {
    type Key = String;

    fn key(&self) -> String {
        self.name.clone()
    }
}
