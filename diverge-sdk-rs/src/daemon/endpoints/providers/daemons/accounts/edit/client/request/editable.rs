//! The daemon record's account as a patch sees it.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::outgoing::Mode;

/// The members of an account on a daemon record an edit may change,
/// as one JSON document: the mode, presented as `null` — the daemon
/// never answers a credential, so a patch reads nothing of the mode
/// there is — and required in the result: a result with `null` is the
/// edit's error, and the daemon changes nothing. The account's name
/// does not change.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// How to authenticate there as this account from now on: see
    /// [`Mode`]. `null` as presented; given whole by the patch.
    #[serde(default)]
    pub mode: Option<Mode>,
}
