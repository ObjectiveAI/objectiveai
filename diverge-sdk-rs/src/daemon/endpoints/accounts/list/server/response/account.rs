//! One account, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::accounts::Told;

/// One account: its name if it has one, its credential without the key
/// if it has one — at least one of the two is there — what it is for,
/// the roles it holds, who is connected as it now, its tags, when it
/// was created and by whom. A list sends them oldest created first,
/// which is the order credentials are tried.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Account {
    /// The name, if the account has one: what a container's `account`
    /// names. Absent, none, and the account dials in alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The credential, without its key, if the account has one: see
    /// [`Told`]. Absent, none, and no client dials in as it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<Told>,
    /// What the account is for, as its create or edit said. Absent,
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The roles it holds, by name, sorted bytewise. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// The identities of the clients connected as it now, in no order:
    /// for a key credential, its one identity or none; for a hook,
    /// every identity the hook has named a connection to. Absent when
    /// empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connected: Vec<String>,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::accounts::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::accounts::untag) has not
    /// taken off. Absent when empty.
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
