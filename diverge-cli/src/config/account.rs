//! One account of a daemon's, as the document holds it.

use serde::{Deserialize, Serialize};

/// An account the CLI may dial a daemon as: the identity it is, one of
/// the root's [`identities`](diverge_sdk::identities), whose token the
/// CLI presents — the daemon's account is that `(issuer, subject)`. The
/// name here is the document's own, and the document holds no token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// The identity's issuer: the one the CLI presents its token from.
    pub issuer: String,
    /// The identity's subject at that issuer.
    pub subject: String,
}
