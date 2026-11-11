//! Which agents the filter's `active` lets through.

use serde::{Deserialize, Serialize};

/// The `active` member of the agents filter: now, or ever.
///
/// Untagged on the wire: a boolean or one of two words, the three
/// shapes never colliding.
///
/// - `true`: active now, a loop running in it; `false`: not active
///   now.
/// - `"ever"`: active at some time — a container was started for it,
///   which is what gives it a `last_active`, whether or not it is
///   active now; `"never"`: never active, exactly the agents that have
///   never run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
pub enum Active {
    /// Whether a loop runs in it now: `true` or `false`.
    Now(bool),
    /// Whether it has ever run: `"ever"` or `"never"`.
    History(History),
}

/// The two words of [`Active`]: whether an agent has ever been
/// active. Snake case on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum History {
    /// Active at some time: `"ever"`.
    Ever,
    /// Never active: `"never"`.
    Never,
}

impl Active {
    /// Whether an agent passes: `active` is whether a loop runs in it
    /// now, `ever` whether it has ever been active.
    pub fn test(self, active: bool, ever: bool) -> bool {
        match self {
            Active::Now(wanted) => active == wanted,
            Active::History(History::Ever) => ever,
            Active::History(History::Never) => !ever,
        }
    }
}
