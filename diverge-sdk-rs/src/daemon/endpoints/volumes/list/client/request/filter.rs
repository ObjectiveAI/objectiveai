//! What narrows a list of volumes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;
use crate::provider::endpoints::volumes::Mode;

/// The filter over volumes: every member optional, and every one given
/// a condition a volume passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends. A filter with no member given passes every
/// volume.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a volume that is any one of
/// them, or is held by any one of them. An empty list is absent, and
/// matches every volume.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Held by any one of these providers: see [`Identity`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<Identity>,
    /// Any one of these names, on whichever provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// In any one of these modes: see [`Mode`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<Mode>,
    /// Whether some agent or tool of the daemon's names it in its
    /// mounts — what a
    /// [`delete`](crate::daemon::endpoints::volumes::delete) answers
    /// `InUse` for — `true`, or none, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mounted: Option<bool>,
    /// Every one of these among the volume's tags, as
    /// [`tag`](crate::daemon::endpoints::volumes::tag) put them. Absent
    /// when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the volume's tags, as
    /// [`tag`](crate::daemon::endpoints::volumes::tag) put them; with
    /// `all_tags`, both hold. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
}
