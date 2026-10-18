//! One value of a vault key, its value apart.

use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

/// One of a key's values as a list reports it: its number and who
/// holds it, and never the value, which a
/// [`get`](crate::daemon::endpoints::vault::get) alone answers as a
/// [`Slot`](crate::daemon::endpoints::vault::get::server::response::Slot).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct SlotState {
    /// Which of the key's values this is: its number, `0` for the
    /// first appended, and so on.
    pub slot: u32,
    /// Who holds it, when a run does: the record — the agent, the
    /// standalone tool, or the dependency tool — whose container took
    /// it as a `rotating` secret and has not given it back. Absent, no
    /// one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<Creator>,
}
