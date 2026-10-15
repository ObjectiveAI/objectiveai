//! The accepting daemon, as the provider knows it.

use serde::{Deserialize, Serialize};

/// What the provider answers an accept with: the identity the
/// daemon's connection is authorized under, which is what a connector
/// names it by, and what the daemon names this provider by when it
/// tells a connector where a container of its runs. The daemon does
/// not choose it and may not know it until here: an outgoing
/// connection presented a credential the provider mapped to it, an
/// incoming one was dialled by the provider under a name of the
/// provider's.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Accepting {
    /// The identity.
    pub identity: String,
}
