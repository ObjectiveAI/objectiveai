//! One entry of an agent's log.

use serde::{Deserialize, Serialize};

use super::Kept;

/// One entry: its place in the log, when it was kept, and what it
/// holds. Whose message a user part is, its own `key` says.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// The item's id: counts up by one per item of the agent's log,
    /// from `1`, and never repeats. What a query's `after` names.
    pub id: u64,
    /// When the daemon kept the item, as an RFC 3339 timestamp with a
    /// UTC offset, `2026-09-21T15:04:05.123456Z`: a string, so a
    /// reader with no clock type reads it as it is.
    pub at: String,
    /// What the item holds.
    pub kept: Kept,
}
