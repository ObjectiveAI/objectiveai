//! The `keep_alive` records: nothing, on schedule.

use serde::{Deserialize, Serialize};

/// A `type: "keep_alive"` record: a transport heartbeat carrying
/// nothing at all. In the pinned source it is written only by the
/// remote transports, never by plain print-mode stdout — but it is a
/// member of the stdout union, and a reader of the union reads all
/// of it.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct KeepAlive {}
