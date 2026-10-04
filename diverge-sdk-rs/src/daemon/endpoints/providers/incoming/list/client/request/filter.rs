//! What narrows a list of judges, and what a permission over them
//! reaches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use super::Kind;

/// The filter over judges: every member optional, and every one given a
/// condition a judge passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends, and its program transforms what passes. In
/// the daemon's own tools, as a
/// [permission](crate::daemon::daemon_tools), the same filter says
/// which judges the tool reaches, and its program is a test: a judge
/// passes when the program, run with the judge as its input, yields
/// first a value that is neither `false` nor `null`. A filter with no
/// member given passes every judge.
///
/// # Any one of
///
/// A member that lists candidates matches a judge that is any one of
/// them, or was made by any one of them. An empty list is absent, and
/// matches every judge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Key judges naming any one of these identities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<String>,
    /// Hook judges of any one of these resources, by id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authorize_hooks: Vec<String>,
    /// Of any one of these forms, `key` or `authorize_hook`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<Kind>,
    /// Whether a provider is connected through it now: `true`, or none,
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
    /// matching judge — one
    /// [`Incoming`](crate::daemon::endpoints::providers::incoming::list::server::response::Incoming)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. Absent, the judges come back as they are. The daemon does
    /// not read the program beyond running it; one that will not
    /// compile, or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}
