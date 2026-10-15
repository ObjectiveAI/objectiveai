//! The outgoing provider as a patch sees it.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::outgoing::Mode;

/// The members of an outgoing provider an edit may change, as one
/// JSON document: the mode, presented as `null` — the daemon never
/// answers a credential, so a patch reads nothing of the mode there
/// is — and required in the result: a result with `null` is the edit's
/// error, and the daemon changes nothing. The address does not change:
/// it is the identity, and a provider at another address is another
/// add.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// How to authenticate there from now on: see [`Mode`]. `null` as
    /// presented; given whole by the patch.
    #[serde(default)]
    pub mode: Option<Mode>,
}
