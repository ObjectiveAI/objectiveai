//! What a grant over resources reaches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::resources::Kind;

/// The filter over resources, read as a test: every member optional,
/// and every one given a condition a resource passes or does not.
/// [`resources::list`](crate::daemon::endpoints::resources::list) takes
/// no filter, so this one is the grant's own, over the members a
/// [`Listed`](crate::daemon::endpoints::resources::list::server::response::Listed)
/// resource has. A filter with no member given passes every resource.
///
/// # Any one of
///
/// A member that lists candidates passes a resource that is any one of
/// them. An empty list is absent, and passes every resource.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these ids: the hashes their uploads answered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// Of any one of these kinds, `file` or `directory`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<Kind>,
    /// The earliest `created` to pass, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to pass, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
    /// A jq program, as the `jq` command takes one, run with the
    /// resource — one
    /// [`Listed`](crate::daemon::endpoints::resources::list::server::response::Listed)
    /// as JSON — as its input: the resource passes when the first value
    /// it yields is neither `false` nor `null`. Absent, every resource
    /// passes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}
