//! The incoming credential as a patch sees it.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::incoming::Credential;

/// The members of a credential of incoming providers an edit may
/// change, as one JSON document: the credential without its key — the
/// identity it names and the address it is accepted from — as the
/// daemon lists it, and as the result must be again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// The credential as it is to be, without its key: see
    /// [`Credential`].
    pub credential: Credential,
}
