//! The accepting daemon, as the provider knows it.

use serde::{Deserialize, Serialize};

use crate::shared::Identity;

/// What the provider answers an accept with: the identity the
/// daemon's connection is authorized under — the `(issuer, subject)`
/// of the token the daemon presented in the handshake — which is what
/// a connector names it by.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Accepting {
    /// The identity.
    pub identity: Identity,
}
