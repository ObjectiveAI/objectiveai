//! One role, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::accounts::Reference;
use crate::daemon::grant::Grant;

/// One role: its name, what it is for, its grants, the accounts that
/// hold it, its tags, when it was created and by whom. A list sends
/// them oldest created first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Role {
    /// The name.
    pub name: String,
    /// What the role is for, as its create or edit said. Absent, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The grants, as the create or the last edit gave them, in that
    /// order: see [`Grant`]. Absent when empty, and then the role
    /// allows nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grants: Vec<Grant>,
    /// The accounts that hold it, in no order, each by its name when it
    /// has one and otherwise by its credential: see
    /// [`Reference`](crate::daemon::endpoints::accounts::Reference).
    /// Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<Reference>,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::roles::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::roles::untag) has not taken
    /// off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// When the create made it. On the wire an RFC 3339 timestamp in
    /// UTC.
    pub created: DateTime<Utc>,
    /// Who created it: the client, over an endpoint, or the agent or
    /// the tool of the client's that did so through the daemon. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker.
    pub creator: Creator,
}
