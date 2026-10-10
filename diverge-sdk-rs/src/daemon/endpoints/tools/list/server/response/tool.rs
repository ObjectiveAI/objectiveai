//! One tool, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::key;
use super::Origin;

/// One tool of the caller's: what the daemon knows of it.
///
/// Everything here the daemon holds for the tool's life, so a list
/// costs no more than the tools it names. What a created tool was
/// made from is its template's, named by id, and its mounts are the
/// create's, and neither is repeated here; what a connected tool is
/// made from is the other daemon's, and unknown here; what a dependency
/// tool was made from is the template its agent's program declared,
/// carried whole on its origin, since it is on record nowhere else.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// The name, as its create or its connect gave it, if it gave
    /// one; absent for a tool made with none, which is reached by its
    /// template and its index, or by the daemon and tool it joined,
    /// alone, and absent for a dependency, which has none and is
    /// reached by its agent and its template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Where it comes from, and what the daemon knows of its
    /// container — with the index that names a record once and for
    /// all, and the agent and template of a dependency: see
    /// [`Origin`].
    pub origin: Origin,
    /// Who made it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker; the maker's own maker is on the maker's list item.
    pub creator: Creator,
    /// When the create or the connect made it.
    pub created: DateTime<Utc>,
    /// Whether the tool is active now: for a created tool, its
    /// container is running; for a connected one, the daemon holds a
    /// connect scope on it. Either is so while an agent it is
    /// attached to is active and the container is there. A
    /// dependency tool is listed only while it runs, and is always
    /// active.
    pub active: bool,
    /// When that last changed: the time the container started, if it
    /// runs; the time it last stopped, if it does not; absent for a
    /// tool that has never run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
    /// The agents it is attached to, each by its
    /// [`key`](crate::daemon::key) — its template and its index, with
    /// its name beside when it has one — in the order they were
    /// attached; empty for a tool attached nowhere; for a dependency
    /// tool, the one agent it was deployed for. The agents list
    /// reports the same attachments from the other side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<key::Agent>,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::tools::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::tools::untag) has not taken
    /// off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
