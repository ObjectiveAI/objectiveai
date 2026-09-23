//! One entry of an agent's log.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Kept;

/// One entry: its place in the log, when it was kept, and what it
/// holds — a chunk's own members flattened beside the two of the
/// log's, so an item reads as the chunk it is with `logs_index` and
/// `created` added; or an error, whole, under `error`. Whose message
/// a user part is, its own `key` says.
///
/// `logs_index` rather than `id`, because a tool call and a tool
/// response carry an `id` of their own, and flattening would put the
/// two in one object. `created` is nobody else's, and `error` is no
/// chunk's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// The item's index: counts up by one per item of the agent's
    /// log, from `1`, and never repeats. What the request's
    /// `logs_index_from` and `logs_index_to` span.
    pub logs_index: u64,
    /// When the daemon kept the item. On the wire an RFC 3339
    /// timestamp in UTC, `2026-09-21T15:04:05.123456Z`. What the
    /// request's `created_from` and `created_to` span.
    pub created: DateTime<Utc>,
    /// What the item holds, its members beside these.
    #[serde(flatten)]
    pub kept: Kept,
}
