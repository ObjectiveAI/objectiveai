//! One dependency template, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::key;
use crate::shared::containers::dependencies::Template;
use super::Run;

/// One dependency template of the caller's: its id, when it was made,
/// who made it, its tags, the template whole, the agents that depend
/// on it, and the tools running from it. The tags are the caller's
/// and not the template's: not in it, and not in its hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listed {
    /// The id: the lowercase hexadecimal SHA-256 of the template's
    /// canonical bytes, as
    /// [`dependencies`](crate::daemon::endpoints::dependencies) states
    /// it. An agent calls a tool running from the template by the
    /// id's first eight characters, as a prefix.
    pub id: String,
    /// When the first registration that declared it made it. On the
    /// wire an RFC 3339 timestamp in UTC. A template declared again —
    /// held already, or made anew after a delete — keeps it, as it
    /// keeps its creator.
    pub created: DateTime<Utc>,
    /// Who made it: the agent whose registration first declared it.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker; the maker's own maker is on the maker's list item. A
    /// template declared again — held already, or made anew after a
    /// delete — is the first maker's still.
    pub creator: Creator,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::dependencies::tag) put on
    /// it and
    /// [`untag`](crate::daemon::endpoints::dependencies::untag) has
    /// not taken off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// The template, as the program declared it: the bytes the id is
    /// the hash of. See [`Template`].
    pub template: Template,
    /// The agents that depend on it now — every agent whose LAST
    /// registration declared it, in the order they first did — each
    /// by its [`key`](crate::daemon::key). An agent whose last
    /// registration left it out is not among them, though a tool of
    /// its may run from the template still, until its run ends.
    /// Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<key::Agent>,
    /// The tools running from it now, one [`Run`] each: what a
    /// [`delete`](crate::daemon::endpoints::dependencies::delete)
    /// answers `InUse` for. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<Run>,
}
