//! What narrows a list of credentials.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

/// The filter over the credentials of incoming providers: every member
/// optional, and every one given a condition a credential passes or
/// does not. In a [list request](super::Frame) it is flattened into the
/// request and narrows what the daemon sends. A filter with no member
/// given passes every credential.
///
/// # Any one of
///
/// A member that lists candidates matches a credential that is any one
/// of them, or was added by any one of them. An empty list is absent,
/// and matches every credential.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Naming any one of these identities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<String>,
    /// Whether a provider is connected through it now: `true`, or none,
    /// `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    /// Added by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then added by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
}
