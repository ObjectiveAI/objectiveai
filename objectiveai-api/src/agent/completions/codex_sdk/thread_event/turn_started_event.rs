use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnStartedEventType {
    #[serde(rename = "turn.started")]
    TurnStarted,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnStartedEvent {
    pub r#type: TurnStartedEventType,
}
