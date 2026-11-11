//! One entry of a command log, with its place and time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Item;

/// One entry: its place in the log, when it was kept, and the
/// [`Item`] it holds, flattened beside them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct ItemWrapper {
    /// The item's index: counts up by one per item, from `1`, and never
    /// repeats.
    pub logs_index: u64,
    /// When the daemon kept the item, an RFC 3339 timestamp in UTC.
    pub created: DateTime<Utc>,
    /// The item, its members beside these.
    #[serde(flatten)]
    pub item: Item,
}
