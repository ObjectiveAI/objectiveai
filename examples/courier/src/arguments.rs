//! What the courier takes as its arguments: nothing.

use serde::{Deserialize, Serialize};

/// The arguments a courier tool is made with.
///
/// None: the courier is the same for every caller, and what it keeps
/// is told apart by the template the tool was made from. `{}` is the
/// one value; any member is refused, so a misspelled setting is never
/// silently ignored.
///
/// # How to run it
///
/// As ONE `service`-mode standalone tool that every agent taking part
/// is attached to. The mail lives in the caller's Postgres, and only
/// agents attached to the same courier share it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Arguments {}
