use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThreadErrorEventType {
    Error,
}

/// Top-level fatal error from the event stream. Distinct from
/// [`super::TurnFailedEvent`]: that one signals a turn that ran but failed,
/// while this one signals the stream itself terminated abnormally.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadErrorEvent {
    pub r#type: ThreadErrorEventType,
    pub message: String,
}
