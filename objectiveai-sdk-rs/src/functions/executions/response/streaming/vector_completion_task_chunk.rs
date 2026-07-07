use crate::agent::completions::response::streaming::AgentCompletionIds;
use crate::{error, vector};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "functions.executions.response.streaming.VectorCompletionTaskChunk"
)]
pub struct VectorCompletionTaskChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub task_index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_u64)]
    pub task_path: Vec<u64>,
    /// The messages dispatched for this vector-completion task.
    /// Populated ONLY on the FIRST chunk of the task; [`push`](Self::push)
    /// keeps the first value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(default)]
    pub request_messages:
        Option<Vec<crate::agent::completions::message::Message>>,
    /// The response options (choices) voted on for this
    /// vector-completion task. Populated ONLY on the FIRST chunk of the
    /// task; [`push`](Self::push) keeps the first value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(default)]
    pub request_choices:
        Option<Vec<crate::agent::completions::message::RichContent>>,
    #[serde(flatten)]
    pub inner: vector::completions::response::streaming::VectorCompletionChunk,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}

impl AgentCompletionIds for VectorCompletionTaskChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> + Send {
        self.inner.agent_completion_ids()
    }
}

impl VectorCompletionTaskChunk {
    pub fn push(&mut self, other: &VectorCompletionTaskChunk) {
        self.inner.push(&other.inner);
        if let Some(error) = &other.error {
            self.error = Some(error.clone());
        }
        // First chunk wins: `request_messages` and `request_choices`
        // ride only the task's first chunk.
        if self.request_messages.is_none() {
            if let Some(request_messages) = &other.request_messages {
                self.request_messages = Some(request_messages.clone());
            }
        }
        if self.request_choices.is_none() {
            if let Some(request_choices) = &other.request_choices {
                self.request_choices = Some(request_choices.clone());
            }
        }
    }
}
