//! What narrows a list of events.

use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

/// The filter over events: every member optional, and every one given
/// a condition an event passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends. A filter with no member given passes every
/// event.
///
/// # Any one of, on either side
///
/// A member that lists candidates matches an event whose own member
/// is any one of them; `parties` matches an event whose sender OR
/// whose receiver is any one of them, so one member names a party's
/// whole neighbourhood. Every member given holds at once. An empty
/// list is absent, and matches every event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Filter {
    /// Any one of these on either side, as the sender or as the
    /// receiver: see [`Creator`](crate::daemon::creator::Creator).
    /// Absent when empty, and then anybody on either side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parties: Vec<Creator>,
    /// Sent by any one of these: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when
    /// empty, and then sent by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub senders: Vec<Creator>,
    /// Received by any one of these: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when
    /// empty, and then received by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receivers: Vec<Creator>,
}
