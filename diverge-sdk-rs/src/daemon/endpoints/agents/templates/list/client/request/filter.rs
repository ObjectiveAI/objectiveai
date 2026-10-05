//! What narrows a list of agent templates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

/// The filter over agent templates: every member optional, and every
/// one given a condition a template passes or does not. In a
/// [list request](super::Frame) it is flattened into the request and
/// narrows what the daemon sends, and its program transforms what
/// passes.
/// A filter with no member given passes every template.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a template that is any one
/// of them, or was made by any one of them; `all_tags` a
/// template that carries every one of them, `any_tags` one that
/// carries any one of them. An empty list is absent, and matches
/// every template.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Whether some agent of the caller's was made from it — what a
    /// [`delete`](crate::daemon::endpoints::agents::templates::delete)
    /// answers `InUse` for — `true`, or none, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_use: Option<bool>,
    /// Every one of these among the template's tags, as
    /// [`tag`](crate::daemon::endpoints::agents::templates::tag) put
    /// them. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the template's tags, as
    /// [`tag`](crate::daemon::endpoints::agents::templates::tag) put
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
    /// A jq program, as the `jq` command takes one, run with each
    /// matching template — one
    /// [`Listed`](crate::daemon::endpoints::agents::templates::list::server::response::Listed)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. So `.id` is every matching template's id, as a string.
    /// Absent, the templates come back as they are. The daemon does not
    /// read the program beyond running it; one that will not compile,
    /// or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}
