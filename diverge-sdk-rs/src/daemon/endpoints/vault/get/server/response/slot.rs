//! One value of a vault key, with its lock.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::daemon::creator::Creator;

/// One of a key's values, as a get answers it: its number, who holds
/// it, and the value — the one place a value is sent.
///
/// A key holds one or more of these, numbered from `0` in the order
/// they were appended, and a run that takes a `rotating` secret from
/// the key takes exactly one of them: locked to the run's record for
/// the run's active period, the value written back and the lock
/// dropped at its inactive, the lock dropped with the value unchanged
/// at the run's end. A `fixed` secret reads a random one and locks
/// nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Slot {
    /// Which of the key's values this is: its number, `0` for the
    /// first appended, and so on. A set that names a number replaces
    /// that value; a delete that names one removes it.
    pub slot: u32,
    /// Who holds it, when a run does: the record — the agent, the
    /// standalone tool, or the dependency tool — whose container took
    /// it as a `rotating` secret and has not given it back. Absent, no
    /// one: it may be taken, set or deleted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<Creator>,
    /// The value, any JSON, as the last set or write-back left it.
    pub value: Value,
}
