//! One identity, as a list answers it.

use serde::{Deserialize, Serialize};

/// One identity signed in on the daemon's host: its issuer, its subject,
/// and the tags the daemon keeps on it. No token is ever answered.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Identity {
    /// The issuer, exactly as its tokens' `iss` names it.
    pub issuer: String,
    /// The subject: its tokens' `sub`.
    pub subject: String,
    /// The tags on it, as [`tag`](crate::daemon::endpoints::identities::tag)
    /// put them, sorted; none where it has none.
    pub tags: Vec<String>,
}
