//! What narrows a list of tools.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use super::Kind;

/// The filter over tools: every member optional, and every one
/// given a condition a tool passes or does not. In a
/// [list request](super::Frame) it is flattened into the request and
/// narrows what the daemon sends, and its program transforms what
/// passes.
/// A filter with no member given passes every tool.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a tool that is any one
/// of them, or was made by any one of them; `all_tags` a
/// tool that carries every one of them, `any_tags` one that
/// carries any one of them. An empty list is absent, and matches
/// every tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these names, as a create or a connect gave them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// A created tool made from any one of these templates, by id; a
    /// connected tool matches none of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Created by the daemon, or connected to somebody else's: the
    /// `kind` an
    /// [`Origin`](crate::daemon::endpoints::tools::list::server::response::Origin)
    /// is tagged with, see [`Kind`]; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    /// Whether active — its container running, or its connect scope
    /// held — `true`, or not, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Attached to every one of these agents, by name. Absent when
    /// empty, and then any attachments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// Every one of these among the tool's tags, as
    /// [`tag`](crate::daemon::endpoints::tools::tag) put them. Absent
    /// when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the tool's tags, as
    /// [`tag`](crate::daemon::endpoints::tools::tag) put them; with
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
    /// matching tool — one
    /// [`Tool`](crate::daemon::endpoints::tools::list::server::response::Tool)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. So `.name` is every matching tool's name, as a string.
    /// Absent, the tools come back as they are. The daemon does not
    /// read the program beyond running it; one that will not compile,
    /// or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}
