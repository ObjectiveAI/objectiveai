//! One template, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::agents::templates::Template;

/// One template of the caller's: its id, when it was made, who made
/// it, its tags, and the template whole. The tags are the caller's and not the
/// template's: not in it, and not in its hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listed {
    /// The id: the template's hash, as [`templates`](crate::daemon::endpoints::agents::templates)
    /// states it.
    pub id: String,
    /// When the create made it.
    pub created: DateTime<Utc>,
    /// Who made it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon's own tools.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker; the maker's own maker is on the maker's list item. A
    /// template made again is the first maker's still.
    pub creator: Creator,
    /// Its tags, sorted bytewise: what [`tag`](crate::daemon::endpoints::agents::templates::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::agents::templates::untag) has not taken off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// The template, as it was handed in: the bytes the id is the
    /// hash of.
    pub template: Template,
}
