//! What narrows a list of roles.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::accounts::Reference;

/// The filter over roles: every member optional, and every one given a
/// condition a role passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends, and its program transforms what passes. A
/// filter with no member given passes every role.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a role that is any one of
/// them, is held by any one of them, or was made by any one of them;
/// `all_tags` a role that carries every one of them, `any_tags` one
/// that carries any one of them. An empty list is absent, and matches
/// every role.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// Held by any one of these accounts: see
    /// [`Reference`](crate::daemon::endpoints::accounts::Reference).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<Reference>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Every one of these among the role's tags, as
    /// [`tag`](crate::daemon::endpoints::roles::tag) put them. Absent
    /// when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the role's tags, as
    /// [`tag`](crate::daemon::endpoints::roles::tag) put them; with
    /// `all_tags`, both hold. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
    /// A jq program, as the `jq` command takes one, run with each
    /// matching role — one
    /// [`Role`](crate::daemon::endpoints::roles::list::server::response::Role)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. Absent, the roles come back as they are. The daemon does
    /// not read the program beyond running it; one that will not
    /// compile, or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}
