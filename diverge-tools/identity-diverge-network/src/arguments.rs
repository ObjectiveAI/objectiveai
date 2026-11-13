//! What the tool takes as its arguments: nothing.

use serde::{Deserialize, Serialize};

/// The arguments an identity.diverge.network tool is made with.
///
/// None: the tool is the same for every caller, and the accounts it
/// keeps are told apart by the template the tool was made from. `{}` is
/// the one value; any member is refused, so a misspelled setting is
/// never silently ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Arguments {}
