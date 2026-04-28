use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreadStartedEventType {
    #[serde(rename = "thread.started")]
    ThreadStarted,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadStartedEvent {
    pub r#type: ThreadStartedEventType,
    pub thread_id: String,
}
