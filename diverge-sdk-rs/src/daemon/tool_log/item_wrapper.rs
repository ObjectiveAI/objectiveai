//! One entry of a tool's log, with its place and time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Item;

/// One entry: its place in the log, when it was kept, and the
/// [`Item`] it holds — the item's own members, `type` among them,
/// flattened beside the log's two.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct ItemWrapper {
    /// The item's index: counts up by one per item of the tool's log,
    /// from `1`, and never repeats. What a request's `logs_index_from`
    /// and `logs_index_to` span, and what an answer names its ask by.
    pub logs_index: u64,
    /// When the daemon kept the item. On the wire an RFC 3339
    /// timestamp in UTC. What a request's `created_from` and
    /// `created_to` span.
    pub created: DateTime<Utc>,
    /// The item, its members beside these.
    #[serde(flatten)]
    pub item: Item,
}
