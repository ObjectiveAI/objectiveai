use crate::{error, vector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorCompletionTaskChunk {
    pub index: u64,
    pub task_index: u64,
    pub task_path: Vec<u64>,
    #[serde(flatten)]
    pub inner: vector::completions::response::streaming::VectorCompletionChunk,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<error::ResponseError>,
}

impl VectorCompletionTaskChunk {
    pub fn push(&mut self, other: &VectorCompletionTaskChunk) {
        self.inner.push(&other.inner);
        match (&mut self.error, &other.error) {
            (None, Some(other_error)) => {
                self.error = Some(other_error.clone());
            }
            _ => {}
        }
    }
}
