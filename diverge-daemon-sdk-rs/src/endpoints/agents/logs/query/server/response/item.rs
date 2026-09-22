//! One entry of an agent's log.

use serde::{Deserialize, Serialize};

use super::Kept;

/// One entry: its place in the log, when it was kept, whose message
/// it is, and what it holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// The item's id: counts up by one per item of the agent's log,
    /// from `1`, and never repeats. What a query's `after` names.
    pub id: u64,
    /// When the daemon kept the item, as an RFC 3339 timestamp with a
    /// UTC offset, `2026-09-21T15:04:05.123456Z`: a string, so a
    /// reader with no clock type reads it as it is.
    pub at: String,
    /// The key of the message the item belongs to: a user part's own,
    /// as its enqueue gave it; absent on everything else, which is
    /// the agent's and not any message's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// What the item holds.
    pub kept: Kept,
}
