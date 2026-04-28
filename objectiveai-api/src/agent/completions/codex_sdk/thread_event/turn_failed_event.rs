use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnFailedEventType {
    #[serde(rename = "turn.failed")]
    TurnFailed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnFailedEvent {
    pub r#type: TurnFailedEventType,
    pub error: super::ThreadError,
}
