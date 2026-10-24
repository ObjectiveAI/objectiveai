//! One way a daemon is reached: a provider, and its name for the
//! daemon.

use serde::{Deserialize, Serialize};

use crate::shared::Identity;

/// One link of a remote daemon: a provider this daemon accepts, by its
/// identity, and the identity the remote daemon is known by at that
/// provider — the `(issuer, subject)` it accepts there under.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Link {
    /// The provider, by its identity.
    pub provider: Identity,
    /// The remote daemon, by its identity at that provider.
    pub daemon: Identity,
}
