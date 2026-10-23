//! What narrows a list of agents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

use super::Active;

/// The filter over agents: every member optional, and every one given a
/// condition a agent passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends. A filter with no member given passes every
/// agent.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a agent that is any one of
/// them, or was made by any one of them; `all_tags` a agent that
/// carries every one of them, `any_tags` one that carries any one of
/// them. An empty list is absent, and matches every agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
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
    /// Whether active now — `true`, or not, `false` — or ever:
    /// `"ever"`, active at some time, or `"never"`, the agents that
    /// have never run. See [`Active`]. Absent, any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<Active>,
    /// Depending on any one of these dependency tool templates, by id:
    /// an agent whose LAST registration declared one of them, as its
    /// `dependencies` say. Absent when empty, and then whatever it
    /// depends on, or nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_tool_templates: Vec<String>,
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
}
