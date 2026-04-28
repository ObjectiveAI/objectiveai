use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnCompletedEventType {
    #[serde(rename = "turn.completed")]
    TurnCompleted,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnCompletedEvent {
    pub r#type: TurnCompletedEventType,
    pub usage: super::Usage,
}
