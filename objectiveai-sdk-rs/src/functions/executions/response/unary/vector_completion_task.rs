use crate::{error, functions::executions::response, vector};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(rename = "functions.executions.response.unary.VectorCompletionTask")]
pub struct VectorCompletionTask {
    pub index: u64,
    pub task_index: u64,
    pub task_path: Vec<u64>,
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
            inner,
            error,
        }: response::streaming::VectorCompletionTaskChunk,
    ) -> Self {
        Self {
            index,
            task_index,
            task_path,
            inner: inner.into(),
            error,
        }
    }
}
