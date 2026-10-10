//! What narrows a list of daemons, and what a permission over them
//! reaches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::agents::logs::server::response::Identity;

/// The filter over daemons: every member optional, and every one given
/// a condition a daemon passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends. A filter with no member given passes every
/// daemon.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a daemon that is any one of
/// them, is linked through any one of them, or was made by any one of
/// them. An empty list is absent, and matches every daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// Linked through any one of these providers, as this daemon names
    /// them: a daemon one of whose links names the provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<Identity>,
    /// Whether this daemon holds a connection to it now: `true`, or
    /// not, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Every one of these among the daemon's tags, as
    /// [`tag`](crate::daemon::endpoints::providers::daemons::tag) put
    /// them. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the daemon's tags, as
    /// [`tag`](crate::daemon::endpoints::providers::daemons::tag) put
    /// them; with `all_tags`, both hold. Absent when empty, and then
    /// any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
}
