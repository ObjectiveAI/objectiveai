use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemStartedEventType {
    #[serde(rename = "item.started")]
    ItemStarted,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ItemStartedEvent {
    pub r#type: ItemStartedEventType,
    pub item: super::super::ThreadItem,
}
