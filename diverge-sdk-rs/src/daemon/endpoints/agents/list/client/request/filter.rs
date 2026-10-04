//! What narrows a list of agents, and what a permission over
//! them reaches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

/// The filter over agents: every member optional, and every one
/// given a condition a agent passes or does not. In a
/// [list request](super::Frame) it is flattened into the request and
/// narrows what the daemon sends, and its program transforms what
/// passes. In the daemon's own tools, as a
/// [permission](crate::daemon::daemon_tools), the same filter says
/// which agents the tool reaches, and its program is a test: a
/// agent passes when the program, run with the agent as its
/// input, yields first a value that is neither `false` nor `null`.
/// A filter with no member given passes every agent.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a agent that is any one
/// of them, or was made by any one of them; `all_tags` a
/// agent that carries every one of them, `any_tags` one that
/// carries any one of them. An empty list is absent, and matches
/// every agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these names, as a create gave them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// Made from any one of these templates, by id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Whether active — a loop running in it — `true`, or not, `false`;
    /// absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Every one of these among the agent's tags, as
    /// [`tag`](crate::daemon::endpoints::agents::tag) put them. Absent
    /// when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the agent's tags, as
    /// [`tag`](crate::daemon::endpoints::agents::tag) put them; with
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
    /// matching agent — one
    /// [`Agent`](crate::daemon::endpoints::agents::list::server::response::Agent)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. So `.name` is every matching agent's name, as a string.
    /// Absent, the agents come back as they are. The daemon does not
    /// read the program beyond running it; one that will not compile,
    /// or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}
