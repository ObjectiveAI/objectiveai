//! One daemon, as this daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::daemons::Link;
use crate::daemon::endpoints::providers::outgoing::Kind;

/// One daemon of the caller's: its name, the kind of its mode without
/// the credential, its links, whether this daemon holds a connection to
/// it now, when it was added and by whom, and its tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Daemon {
    /// The name, as the add gave it: the daemon's identity here.
    pub name: String,
    /// Which mode this daemon authenticates to it in, without the
    /// credential. See [`Kind`].
    pub kind: Kind,
    /// The providers it is reached through, each with the identity it
    /// is known by there: see [`Link`]. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// Whether this daemon holds a connection to it now, through one of
    /// its links.
    pub connected: bool,
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
