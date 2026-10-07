//! One agent template, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::templates::Template;
use diverge_sdk::daemon::endpoints::agents::templates::list::server::response::Listed;

/// A live agent template row: the template whole, its id, its tags,
/// and who first made it.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// The id: the template's hash.
    pub id: String,
    /// The template, as it was handed in.
    pub template: Template,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When the first create made it.
    pub created: DateTime<Utc>,
    /// Who first made it.
    pub creator: Creator,
}

impl Record {
    /// The template as a list reports it.
    pub fn report(&self) -> Listed {
        Listed {
            id: self.id.clone(),
            created: self.created,
            creator: self.creator.clone(),
            tags: self.tags.clone(),
            template: self.template.clone(),
        }
    }
}
