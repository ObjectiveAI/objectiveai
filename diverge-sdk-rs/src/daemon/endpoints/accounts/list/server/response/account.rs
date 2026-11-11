//! One account, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::shared::Identity;
use crate::shared::listing::Keyed;

/// One account: its identity, what it is for, the roles it holds,
/// whether a client is connected as it now, its tags, when it was
/// created and by whom, and whether the daemon's configuration names it
/// an administrator. A list sends them oldest created first, the
/// administrators the configuration names with no account of their
/// own after them.
///
/// An administrator with no account of its own is listed all the same:
/// `admin` true, nothing to say of roles, tags or what it is for, and
/// no `created` or `creator`, since the configuration made it and not a
/// request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Account {
    /// The identity the account is: the `(issuer, subject)` a client's
    /// token establishes.
    pub identity: Identity,
    /// What the account is for, as its create or edit said. Absent,
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The roles it holds, by name, sorted bytewise. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Whether a client is connected as it now.
    pub connected: bool,
    /// When that last changed: the time the first connection now up
    /// was made, if one is; the time the last one ended, if not;
    /// absent for an account never connected as. An administrator with
    /// no account of its own is known only since the daemon started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connected: Option<DateTime<Utc>>,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::accounts::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::accounts::untag) has not
    /// taken off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// When the create made it. On the wire an RFC 3339 timestamp in
    /// UTC. Absent for an administrator with no account of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,
    /// Who created it: the client, over an endpoint, or the agent or
    /// the tool of the client's that did so through the daemon. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker.
    /// Absent for an administrator with no account of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<Creator>,
    /// Whether the daemon's configuration names the identity an
    /// administrator, holding every grant there is. Absent, `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub admin: bool,
}

/// Keyed by the identity, which does not change while it is listed.
impl Keyed for Account {
    type Key = Identity;

    fn key(&self) -> Identity {
        self.identity.clone()
    }
}
