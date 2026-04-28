use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemCompletedEventType {
    #[serde(rename = "item.completed")]
    ItemCompleted,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ItemCompletedEvent {
    pub r#type: ItemCompletedEventType,
    pub item: super::super::ThreadItem,
}
