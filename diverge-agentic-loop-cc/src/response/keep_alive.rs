//! The `keep_alive` records: nothing, on schedule.

use serde::Deserialize;

/// A `type: "keep_alive"` record: a transport heartbeat carrying
/// nothing at all. In the pinned source it is written only by the
/// remote transports, never by plain print-mode stdout — but it is a
/// member of the stdout union, and a reader of the union reads all
/// of it.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub struct KeepAlive {
    /// Always `keep_alive`.
    pub r#type: KeepAliveType,
}

/// The `keep_alive` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum KeepAliveType {
    /// The only value.
    #[default]
    KeepAlive,
}
