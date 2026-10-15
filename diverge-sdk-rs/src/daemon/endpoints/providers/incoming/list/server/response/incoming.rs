//! One credential of incoming providers, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::Status;
use crate::provider::endpoints::capacity;
use crate::daemon::endpoints::providers::incoming::Credential;

/// One credential: the identity it names and the address it is accepted
/// from — never the key — where the daemon stands with the provider
/// connected through it now, when it was added and by whom, and its
/// tags. A list sends them oldest added first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Incoming {
    /// The credential, which carries no key: see [`Credential`].
    /// Flattened, so its members are this object's own.
    #[serde(flatten)]
    pub credential: Credential,
    /// Where the daemon stands with the provider connected through it
    /// now: see [`Status`]. Exactly one is, as the credential's
    /// identity, or none — a credential admits one connection at a
    /// time.
    pub status: Status,
    /// The connected provider's latest answer to the daemon's
    /// `capacity` ask, as the daemon cached it: how much the daemon
    /// may still use of it, see
    /// [`Frame`](crate::provider::endpoints::capacity::server::response::Frame).
    /// Absent until the provider has answered once, and whenever the
    /// status is not `connected`. The daemon asks again every
    /// `capacity_poll_seconds` of its configuration and after every
    /// act of its own that changes the answer; a list kept open hears
    /// each new answer as the item changed. A member the provider's
    /// capabilities exclude is not read, as the provider wire has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<capacity::server::response::Frame>,
    /// When the add made it. On the wire an RFC 3339 timestamp in UTC.
    pub created: DateTime<Utc>,
    /// Who added it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker.
    pub creator: Creator,
    /// The tags on it, as
    /// [`tag`](crate::daemon::endpoints::providers::incoming::tag) put
    /// them. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
