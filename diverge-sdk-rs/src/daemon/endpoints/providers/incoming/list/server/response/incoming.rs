//! One credential of incoming providers, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::incoming::Credential;

/// One credential: the identity it names and the address it is accepted
/// from — never the key — whether a provider is connected through it
/// now, when it was added and by whom. A list sends them oldest added
/// first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Incoming {
    /// The credential, which carries no key: see [`Credential`].
    /// Flattened, so its members are this object's own.
    #[serde(flatten)]
    pub credential: Credential,
    /// Whether a provider is connected through it now: the one that is,
    /// as the credential's identity, since one credential admits one
    /// identity.
    pub connected: bool,
    /// When the add made it. On the wire an RFC 3339 timestamp in UTC.
    pub created: DateTime<Utc>,
    /// Who added it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker.
    pub creator: Creator,
}
