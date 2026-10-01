//! One resource, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::resources::Kind;

/// One resource of the caller's: its id, its kind, when it was
/// uploaded, and its size.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Listed {
    /// The id: the resource's hash, as
    /// [`resources`](crate::daemon::endpoints::resources) states it.
    pub id: String,
    /// A file, or a directory.
    pub kind: Kind,
    /// When the upload held it.
    pub created: DateTime<Utc>,
    /// How many bytes it holds: the file's length, or the sum of a
    /// directory's files'.
    pub bytes: u64,
}
