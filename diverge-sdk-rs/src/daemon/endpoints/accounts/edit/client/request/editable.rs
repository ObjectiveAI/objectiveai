//! The account as a patch sees it.

use serde::{Deserialize, Serialize};

/// The members of an account an edit may change, as one JSON
/// document: what a patch is applied to, and what its result must be
/// again — every member present, a member the patch takes away read
/// as absent or empty, and no member this type does not have. The
/// identity is the account, and is not here: another identity is
/// another account.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// What the account is for, in words. `null`, none.
    #[serde(default)]
    pub description: Option<String>,
    /// The roles it holds, by name, each one the daemon has and the
    /// caller holds the `grant` grant over. Empty, the account may do
    /// nothing.
    #[serde(default)]
    pub roles: Vec<String>,
}
