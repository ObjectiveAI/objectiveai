//! One dependency tool, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;
use crate::daemon::creator::Creator;
use crate::daemon::key;
use crate::shared::containers::dependencies::Template;
use super::{Attached, Run};

/// One dependency tool of the caller's: its template and its owner,
/// which together name it, when it was made, who made it, its tags,
/// the template whole, the agents that depend on it, and its run, if
/// its container is up. The tags are the tool's and not the
/// template's: not in it, and not in its hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Listed {
    /// The template, by id: the lowercase hexadecimal SHA-256 of its
    /// canonical bytes, as
    /// [`dependency_tools`](crate::daemon::endpoints::dependency_tools)
    /// states it. An agent calls the tool's MCP tools by the id's
    /// first eight characters, as a prefix.
    pub id: String,
    /// Whose the tool is, as the template's scope decides: the one
    /// agent, the one agent template, or nobody. See [`key::Owner`].
    pub owner: key::Owner,
    /// Its number among all dependency tools ever made from the
    /// template, whatever their owners: the first is `1`, and no
    /// number is given twice. The template and the index name the tool
    /// once and for all, as an
    /// [`events`](crate::daemon::endpoints::events) party does.
    pub index: u64,
    /// When the first registration that declared the template under
    /// this owner made the tool. On the wire an RFC 3339 timestamp in
    /// UTC.
    pub created: DateTime<Utc>,
    /// Who made it: the agent whose registration first declared the
    /// template under this owner. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker;
    /// the maker's own maker is on the maker's list item.
    pub creator: Creator,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::dependency_tools::tag) put on
    /// it and
    /// [`untag`](crate::daemon::endpoints::dependency_tools::untag) has
    /// not taken off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// The template, as the program declared it: the bytes the id is
    /// the hash of. See [`Template`].
    pub template: Template,
    /// The agents that depend on it now — every agent whose LAST
    /// registration declared the template under this owner, in the
    /// order they first did, each with when it last did: see
    /// [`Attached`]. At most one for an instance tool. Absent when
    /// empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<Attached>,
    /// Its run, while its container is up: see [`Run`]. What a
    /// [`delete`](crate::daemon::endpoints::dependency_tools::delete)
    /// answers `InUse` for. Absent while it does not run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<Run>,
}

/// Keyed by the template's id and the owner, which together name the
/// dependency tool.
impl Keyed for Listed {
    type Key = (String, key::Owner);

    fn key(&self) -> (String, key::Owner) {
        (self.id.clone(), self.owner.clone())
    }
}
