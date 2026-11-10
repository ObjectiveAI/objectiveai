//! What the alarm tool takes as its arguments: nothing.

use serde::{Deserialize, Serialize};

/// The arguments an alarm tool is made with.
///
/// None: every alarm names the agent that set it, and the tool is the
/// same whoever calls it. `{}` is the one value; any member is
/// refused, so a misspelled setting is never silently ignored.
///
/// # How it runs
///
/// As a dependency tool: a service, declared by the image of every
/// agent that sets alarms, granted `message` over the agents attached
/// to it. A service, because it keeps time between the agents' runs;
/// any scope serves, since every alarm wakes only its own agent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Arguments {}
