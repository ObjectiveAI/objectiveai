//! What narrows a list of connected tools.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::daemons::DaemonId;

/// The filter over connected tools: every member optional, and every
/// one given a condition a tool passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends. A filter with no member given passes every
/// tool.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a tool that is any one of
/// them, is on any one of them, or was made by any one of them;
/// `all_tags` a tool that carries every one of them, `any_tags` one
/// that carries any one of them. An empty list is absent, and matches
/// every tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Filter {
    /// Any one of these names, as a create gave them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// On any one of these daemons, by the ID of this daemon's record
    /// of each, as the create named it — the tool's `daemon`, the
    /// first of its route's daemons where it has a route. Empty, any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub daemons: Vec<DaemonId>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Whether active — a call or a read the daemon routed into it in
    /// flight — `true`, or not, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Whether deployed — its connect held — `true`, or not, `false`;
    /// absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployed: Option<bool>,
    /// Attached directly to every one of these agents, by name. Absent
    /// when empty, and then any attachments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attached_agent_instances: Vec<String>,
    /// Attached to every one of these agent templates, by id. Absent
    /// when empty, and then any attachments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attached_agent_templates: Vec<String>,
    /// Every one of these among the tool's tags, as
    /// [`tag`](crate::daemon::endpoints::connected_standalone_tools::tag)
    /// put them. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the tool's tags, as
    /// [`tag`](crate::daemon::endpoints::connected_standalone_tools::tag)
    /// put them; with `all_tags`, both hold. Absent when empty, and then
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
