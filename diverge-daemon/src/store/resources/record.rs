//! One resource, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::resources::Kind;
use diverge_sdk::daemon::endpoints::resources::list::server::response::Listed;

/// A live resource row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The id: the hash of the bytes.
    pub id: String,
    /// A file, or a directory of files.
    pub kind: Kind,
    /// What it is for, as the latest upload said.
    pub description: String,
    /// The file's length, or the sum of the directory's files'.
    pub bytes: u64,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was first held.
    pub created: DateTime<Utc>,
    /// Who first held it.
    pub creator: Creator,
}

impl Record {
    /// The resource as a list reports it.
    pub fn report(&self) -> Listed {
        Listed {
            id: self.id.clone(),
            kind: self.kind,
            description: self.description.clone(),
            created: self.created,
            creator: self.creator.clone(),
            tags: self.tags.clone(),
            bytes: self.bytes,
        }
    }
}

/// A kind as the column holds it.
pub fn kind_text(kind: Kind) -> &'static str {
    match kind {
        Kind::File => "file",
        Kind::Directory => "directory",
    }
}
