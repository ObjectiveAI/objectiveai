//! One vault key, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::vault::get::server::response;
use diverge_sdk::daemon::endpoints::vault::list::server::response::Key;
use diverge_sdk::daemon::key;
use serde_json::Value;

/// A vault row, with the records binding it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The key, which never changes.
    pub key: String,
    /// The value, as the last set gave it.
    pub value: Value,
    /// The agents binding it, in no order.
    pub agents: Vec<key::Agent>,
    /// The standalone tools binding it, in no order.
    pub standalone_tools: Vec<key::Tool>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When the first set made it.
    pub created: DateTime<Utc>,
    /// Who made it.
    pub creator: Creator,
}

impl Entry {
    /// The key as a list reports it: everything but the value.
    pub fn report(&self) -> Key {
        Key {
            key: self.key.clone(),
            agents: self.agents.clone(),
            standalone_tools: self.standalone_tools.clone(),
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
        }
    }

    /// The key as a get answers it: the report, with the value.
    pub fn report_with_value(&self) -> response::Entry {
        response::Entry {
            key: self.key.clone(),
            value: self.value.clone(),
            agents: self.agents.clone(),
            standalone_tools: self.standalone_tools.clone(),
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
        }
    }
}
