//! One resource, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::resources::Kind;

/// One resource: its id, its kind, what it is in words, when it was
/// first held and by whom, its tags, and its size. The tags are the
/// caller's and not the resource's: not in it, and not in its hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Listed {
    /// The id: the resource's hash, as
    /// [`resources`](crate::daemon::endpoints::resources) states it.
    pub id: String,
    /// A file, or a directory.
    pub kind: Kind,
    /// What the resource is, in words: the description of its latest
    /// upload, or of the transfer that last made it.
    pub description: String,
    /// When the first upload, or the first transfer, held it. On the
    /// wire an RFC 3339 timestamp in UTC. A resource held again — an
    /// upload answered `Exists`, or the same bytes held anew after a
    /// delete — keeps it, as it keeps its creator.
    pub created: DateTime<Utc>,
    /// Who first held it: the account that uploaded it, or whose
    /// transfer made it, over an endpoint or through the daemon. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker;
    /// a resource held again is the first maker's still.
    pub creator: Creator,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::resources::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::resources::untag) has not
    /// taken off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// How many bytes it holds: the file's length, or the sum of a
    /// directory's files'.
    pub bytes: u64,
}
