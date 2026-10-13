//! One dependency template, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::dependency_tools::list::server::response::{Listed, Run};
use diverge_sdk::daemon::key;
use diverge_sdk::shared::containers::dependencies::Template;

/// A live dependency template row: the template whole, its id, its
/// tags, and the agent whose registration first declared it.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// The id: the template's hash.
    pub id: String,
    /// The template, as the program declared it.
    pub template: Template,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When the first registration made it.
    pub created: DateTime<Utc>,
    /// The agent whose registration first made it.
    pub creator: Creator,
}

impl Record {
    /// The template as a list reports it, with the agents that depend
    /// on it now and the tools running from it now, loaded by the
    /// caller.
    pub fn report(&self, agents: Vec<key::Agent>, runs: Vec<Run>) -> Listed {
        Listed {
            id: self.id.clone(),
            created: self.created,
            creator: self.creator.clone(),
            tags: self.tags.clone(),
            template: self.template.clone(),
            agents,
            runs,
        }
    }
}
