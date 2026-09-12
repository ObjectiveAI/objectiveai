//! A hook that judges the credential.

use serde::{Deserialize, Serialize};

/// A hook that judges the credential itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    /// The hook, by name: the folder `hooks/<name>/` of the
    /// provider's directory, run as [`hook`](crate::hook) provides.
    /// It reads the credential and the peer's address, an
    /// [`Input`](super::authorize_hook::Input), and writes an identity
    /// or a refusal, an [`Output`](super::authorize_hook::Output).
    pub authorize_hook: String,
}
