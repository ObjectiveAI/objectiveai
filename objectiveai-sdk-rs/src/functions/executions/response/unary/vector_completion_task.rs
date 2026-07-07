use crate::{error, functions::executions::response, vector};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(rename = "functions.executions.response.unary.VectorCompletionTask")]
pub struct VectorCompletionTask {
    pub index: u64,
    pub task_index: u64,
    pub task_path: Vec<u64>,
    /// The messages dispatched for this vector-completion task,
    /// carried from the task's first streaming chunk.
    pub request_messages: Vec<crate::agent::completions::message::Message>,
    /// The response options (choices) voted on for this
    /// vector-completion task, carried from the task's first streaming
    /// chunk.
    pub request_choices:
        Vec<crate::agent::completions::message::RichContent>,
    #[serde(flatten)]
    pub inner: vector::completions::response::unary::VectorCompletion,
    pub error: Option<error::ResponseError>,
}

impl From<response::streaming::VectorCompletionTaskChunk>
    for VectorCompletionTask
{
    fn from(
        response::streaming::VectorCompletionTaskChunk {
            index,
            task_index,
            task_path,
            request_messages,
            request_choices,
            inner,
            error,
        }: response::streaming::VectorCompletionTaskChunk,
    ) -> Self {
        Self {
            index,
            task_index,
            task_path,
            request_messages: request_messages.unwrap_or_default(),
            request_choices: request_choices.unwrap_or_default(),
            inner: inner.into(),
            error,
        }
    }
}
