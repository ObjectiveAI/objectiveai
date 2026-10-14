//! What narrows a list of dependency tools.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::reference;
use crate::shared::containers::dependencies::Scope;
use super::Type;

/// The filter over dependency tools: every member optional, and every
/// one given a condition a tool passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends. A filter with no member given passes every
/// tool.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a tool that is of any one
/// of them, owned by any one of them, was made by any one of them, or
/// is of any one of their types or scopes; `agents` a tool every one
/// of them depends on; `all_tags` a tool that carries every one of
/// them, `any_tags` one that carries any one of them. An empty list is
/// absent, and matches every tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Filter {
    /// Of any one of these templates, by id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// Owned by any one of these: an agent, by name or once and for
    /// all, an agent template, or nobody — see [`reference::Owner`].
    /// Absent when empty, and then any owner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<reference::Owner>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator) — the agent whose
    /// registration first made the tool. Absent when empty, and then
    /// made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Of any one of these types, transient or a service: see
    /// [`Type`]. Absent when empty, and then either.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<Type>,
    /// A service whose `scope` is any one of these: see [`Scope`]. A
    /// transient tool has no scope and matches none of them. Absent
    /// when empty, and then any scope, or none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<Scope>,
    /// Whether some agent's last registration declared it — the tool
    /// has `agents` on its list item — `true`, or none did, `false`;
    /// absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depended: Option<bool>,
    /// Depended on by every one of these agents, by name. Absent when
    /// empty, and then by any agents, or none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// Whether its container runs now — the tool has a `run` on its
    /// list item, and a
    /// [`delete`](crate::daemon::endpoints::dependency_tools::delete)
    /// of its template answers `InUse` — `true`, or not, `false`;
    /// absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_use: Option<bool>,
    /// Every one of these among the tool's tags, as
    /// [`tag`](crate::daemon::endpoints::dependency_tools::tag) put
    /// them. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the tool's tags, as
    /// [`tag`](crate::daemon::endpoints::dependency_tools::tag) put
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
}
