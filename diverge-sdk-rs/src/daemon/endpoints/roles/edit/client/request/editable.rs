//! The role as a patch sees it.

use serde::{Deserialize, Serialize};

use crate::daemon::grant::Grant;

/// The members of a role an edit may change, as one JSON document:
/// what a patch is applied to, and what its result must be again —
/// every member present, and no member this type does not have. The
/// name is not among them: a role is reached by its name alone, so a
/// new name is a new role.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// What the role is for, in words. `null`, none.
    #[serde(default)]
    pub description: Option<String>,
    /// The grants, in order: see [`Grant`]. Empty, the role allows
    /// nothing.
    #[serde(default)]
    pub grants: Vec<Grant>,
}
