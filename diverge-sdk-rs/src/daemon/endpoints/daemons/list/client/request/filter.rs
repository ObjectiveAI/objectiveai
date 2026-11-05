//! What narrows a list of remote daemons.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

/// The filter over remote daemons: every member optional, and every one
/// given a condition a daemon passes or does not. In a list request it
/// narrows what the daemon sends; in a grant it is how far the grant
/// reaches. A filter with no member given passes every daemon.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a daemon that is any one of
/// them. An empty list is absent, and matches every daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Filter {
    /// Any one of these names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// The remote at any one of these issuers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issuers: Vec<String>,
    /// Added by any one of these.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Whether this daemon holds a connection to it now: `true`, or
    /// none, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    /// Every one of these among its tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// At least one of these among its tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
    /// Added at or after this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// Added at or before this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
}
