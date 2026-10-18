//! One vault key, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::vault::get::server::response;
use diverge_sdk::daemon::endpoints::vault::list::server::response::{Key, SlotState};
use diverge_sdk::daemon::key;
use serde_json::Value;

/// One value of a vault key: its slot.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// Its number: from `0`, in the order the values were appended.
    pub slot: u32,
    /// The record whose running container holds it now, if one does:
    /// the agent, the standalone tool or the dependency tool. None
    /// while the slot is free.
    pub holder: Option<Creator>,
    /// The value, as the last set or write-back gave it.
    pub value: Value,
}

/// A vault row, with its slots and the records binding it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The key, which never changes.
    pub key: String,
    /// Its values, by slot number ascending.
    pub slots: Vec<Slot>,
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
    /// The key as a list reports it: every slot by number and who
    /// holds it, and never a value.
    pub fn report(&self) -> Key {
        Key {
            key: self.key.clone(),
            slots: self
                .slots
                .iter()
                .map(|slot| SlotState {
                    slot: slot.slot,
                    locked: slot.holder.clone(),
                })
                .collect(),
            agents: self.agents.clone(),
            standalone_tools: self.standalone_tools.clone(),
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
        }
    }

    /// The key as a get answers it: the report, with every value.
    pub fn report_with_value(&self) -> response::Entry {
        response::Entry {
            key: self.key.clone(),
            slots: self
                .slots
                .iter()
                .map(|slot| response::Slot {
                    slot: slot.slot,
                    locked: slot.holder.clone(),
                    value: slot.value.clone(),
                })
                .collect(),
            agents: self.agents.clone(),
            standalone_tools: self.standalone_tools.clone(),
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
        }
    }
}
