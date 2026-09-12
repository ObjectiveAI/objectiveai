//! A hook that judges the credential.

use serde::{Deserialize, Serialize};

/// A hook that judges the credential itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    /// The hook, by name: the folder `hooks/<name>/` of the
    /// provider's directory, run as [`hook`](crate::hook) provides,
    /// and given the credential and the peer's address.
    pub authorize_hook: String,
}
