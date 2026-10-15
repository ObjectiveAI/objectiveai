//! The account as a patch sees it.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::accounts::Credential;

/// The members of an account an edit may change, as one JSON
/// document: what a patch is applied to, and what its result must be
/// again — every member present, a member the patch takes away read
/// as absent or empty, and no member this type does not have. The
/// credential carries no key: the key is minted, never read.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// The name, unique among accounts; another's is the edit's
    /// `InUse`. `null`, the account has no name and dials in alone,
    /// which a container running under it forbids.
    #[serde(default)]
    pub name: Option<String>,
    /// How a client dials in as the account: see [`Credential`]. An
    /// identity another account's credential names is the edit's
    /// `InUse`. `null`, no client dials in as it from then on, which a
    /// client connected as it now forbids.
    #[serde(default)]
    pub credential: Option<Credential>,
    /// What the account is for, in words. `null`, none.
    #[serde(default)]
    pub description: Option<String>,
    /// The roles it holds, by name, each one the daemon has and the
    /// caller holds the `grant` grant over. Empty, the account may do
    /// nothing.
    #[serde(default)]
    pub roles: Vec<String>,
}
