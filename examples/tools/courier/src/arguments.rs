//! What the courier takes as its arguments: nothing.

use serde::{Deserialize, Serialize};

/// The arguments a courier tool is made with.
///
/// None: the courier is the same for every caller, and what it keeps
/// is told apart by the template the tool was made from. `{}` is the
/// one value; any member is refused, so a misspelled setting is never
/// silently ignored.
///
/// # How it runs
///
/// As a dependency tool: a service whose scope is `global`, declared
/// by the image of every agent that takes part. One container, one
/// database, shared by every agent that declares it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Arguments {}
