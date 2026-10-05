//! What narrows a list of outgoing providers, and what a permission
//! over them reaches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::outgoing::Kind;

/// The filter over outgoing providers: every member optional, and every
/// one given a condition a provider passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends, and its program transforms what passes.
/// A filter with no member given passes every provider.
///
/// # Any one of
///
/// A member that lists candidates matches a provider that is any one of
/// them, or was made by any one of them. An empty list is absent, and
/// matches every provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these addresses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<String>,
    /// Dialled in any one of these modes, by kind.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<Kind>,
    /// Whether the daemon holds a connection to it now: `true`, or not,
    /// `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
    /// A jq program, as the `jq` command takes one, run with each
    /// matching provider — one
    /// [`Outgoing`](crate::daemon::endpoints::providers::outgoing::list::server::response::Outgoing)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. Absent, the outgoing providers come back as they are. The
    /// daemon does not read the program beyond running it; one that
    /// will not compile, or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}
