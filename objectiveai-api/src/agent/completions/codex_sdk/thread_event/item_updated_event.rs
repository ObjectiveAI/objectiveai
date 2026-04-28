use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemUpdatedEventType {
    #[serde(rename = "item.updated")]
    ItemUpdated,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ItemUpdatedEvent {
    pub r#type: ItemUpdatedEventType,
    pub item: super::super::ThreadItem,
}
