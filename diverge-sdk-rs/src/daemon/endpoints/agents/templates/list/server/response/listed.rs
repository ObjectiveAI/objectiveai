//! One template, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;
use crate::daemon::creator::Creator;
use crate::daemon::endpoints::agents::templates::Template;
use crate::daemon::key;

/// One template of the caller's: its id, when it was made, who made it,
/// its tags, the tools attached to it, and the template whole. The
/// tags and the attachments are the caller's and not the template's:
/// not in it, and not in its hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Listed {
    /// The id: the template's hash, as
    /// [`templates`](crate::daemon::endpoints::agents::templates)
    /// states it.
    pub id: String,
    /// When the first create made it. On the wire an RFC 3339
    /// timestamp in UTC. A template made again — answered `Exists`,
    /// or made anew after a delete — keeps it, as it keeps its creator.
    pub created: DateTime<Utc>,
    /// Who made it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker; the maker's own maker is on the maker's list item. A
    /// template made again — answered `Exists`, or made anew after a
    /// delete — is the first maker's still.
    pub creator: Creator,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::agents::templates::tag) put on
    /// it and
    /// [`untag`](crate::daemon::endpoints::agents::templates::untag)
    /// has not taken off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// The [`standalone_tools`](crate::daemon::endpoints::standalone_tools)
    /// attached to it by
    /// [`attach_agent_template`](crate::daemon::endpoints::standalone_tools::attach_agent_template),
    /// which every agent made from it has, each by its
    /// [`key`](crate::daemon::key), in the order they were attached;
    /// empty for none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<key::Tool>,
    /// The template, as it was handed in: the bytes the id is the
    /// hash of.
    pub template: Template,
}

/// Keyed by the id.
impl Keyed for Listed {
    type Key = String;

    fn key(&self) -> String {
        self.id.clone()
    }
}
