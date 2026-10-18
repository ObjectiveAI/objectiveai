//! The daemon record as a patch sees it.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::daemons::Link;

/// The members of a daemon record an edit may change, as one JSON
/// document: the links, as the daemon lists them, and as the result
/// must be again. The name does not change, and the accounts are each
/// their own edit's.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// The providers it is reached through, in order: see [`Link`].
    /// Each link's provider is one of the caller's on record, else the
    /// edit's `NoProvider`.
    #[serde(default)]
    pub links: Vec<Link>,
}
