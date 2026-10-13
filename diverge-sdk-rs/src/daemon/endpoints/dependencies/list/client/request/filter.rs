//! What narrows a list of dependency templates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::shared::containers::dependencies::Scope;
use super::Type;

/// The filter over dependency templates: every member optional, and
/// every one given a condition a template passes or does not. In a
/// [list request](super::Frame) it is flattened into the request and
/// narrows what the daemon sends. A filter with no member given passes
/// every template.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches a template that is any one
/// of them, was made by any one of them, or is of any one of them;
/// `agents` a template every one of them depends on; `all_tags` a
/// template that carries every one of them, `any_tags` one that
/// carries any one of them. An empty list is absent, and matches every
/// template.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator) — the agent whose
    /// registration first declared it. Absent when empty, and then
    /// made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Of any one of these types, transient or a service: see
    /// [`Type`]. Absent when empty, and then either.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<Type>,
    /// A service whose `scope` is any one of these: see [`Scope`]. A
    /// transient template has no scope and matches none of them.
    /// Absent when empty, and then any scope, or none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<Scope>,
    /// Whether some agent's last registration declared it — the
    /// template is among the `agents` of its list item — `true`, or
    /// none did, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depended: Option<bool>,
    /// Depended on by every one of these agents, by name. Absent when
    /// empty, and then by any agents, or none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// Whether some tool runs from it now — the template has `runs`
    /// on its list item, and a
    /// [`delete`](crate::daemon::endpoints::dependencies::delete)
    /// answers `InUse` — `true`, or none does, `false`; absent,
    /// either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_use: Option<bool>,
    /// Every one of these among the template's tags, as
    /// [`tag`](crate::daemon::endpoints::dependencies::tag) put
    /// them. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the template's tags, as
    /// [`tag`](crate::daemon::endpoints::dependencies::tag) put
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
