//! The connected tool as a patch sees it.

use serde::{Deserialize, Serialize};

/// The members of a connected tool an edit may change, as one JSON
/// document: what a patch is applied to, and what its result must be
/// again — every member present, a member the patch takes away read as
/// absent, and no member this type does not have. Its name and nothing
/// else: the daemon, the route and the tool it is held from are fixed
/// at its create, and what it is made from is the other daemon's.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// The name, unique across the daemon's standalone and connected
    /// tools together; another's
    /// is the edit's `InUse`. `null`, the tool has no name, and is
    /// reached once and for all only.
    #[serde(default)]
    pub name: Option<String>,
}
