//! The other end, verified.

use serde::{Deserialize, Serialize};

/// The identity the other end's token established: its issuer and its
/// subject, as its [`Authenticator`](super::Authenticator) verified
/// them.
///
/// The one identity of every party: a daemon's account, a provider a
/// daemon is connected to, a daemon a provider serves. On the wire, as
/// JSON, `{"issuer": …, "subject": …}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Peer {
    /// The token's `iss`.
    pub issuer: String,
    /// The token's `sub`.
    pub subject: String,
}

impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}` at `{}`", self.subject, self.issuer)
    }
}
