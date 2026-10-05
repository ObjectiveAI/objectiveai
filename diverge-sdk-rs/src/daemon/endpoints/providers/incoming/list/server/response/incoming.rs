//! One judge of incoming providers, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::incoming::Told;

/// One judge of the caller's: the judge without its key, which
/// providers are connected through it now, when it was added and by
/// whom. A list sends them oldest added first, which is the order they
/// are tried.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Incoming {
    /// The judge, without its key: see [`Told`].
    pub judge: Told,
    /// The identities of the providers connected through it now, in no
    /// order: for a key judge, its one identity or none; for a hook
    /// judge, every identity the hook has named a connection to. Absent
    /// when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connected: Vec<String>,
    /// When the add made it. On the wire an RFC 3339 timestamp in UTC.
    pub created: DateTime<Utc>,
    /// Who added it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker.
    pub creator: Creator,
}
