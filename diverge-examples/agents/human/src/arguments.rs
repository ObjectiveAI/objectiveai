//! What the agent takes as its arguments: nothing.

use serde::{Deserialize, Serialize};

/// The arguments a human agent is made with.
///
/// None: the agent is whoever answers on its frontend, and its tools
/// are whichever are attached to it. `{}` is the one value; any member
/// is refused, so a misspelled setting is never silently ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Arguments {}
