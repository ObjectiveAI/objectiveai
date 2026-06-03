use crate::agent::completions::response::streaming::AgentCompletionIds;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(untagged)]
#[schemars(rename = "functions.executions.response.streaming.TaskChunk")]
pub enum TaskChunk {
    #[schemars(title = "FunctionExecution")]
    FunctionExecution(super::FunctionExecutionTaskChunk),
    #[schemars(title = "VectorCompletion")]
    VectorCompletion(super::VectorCompletionTaskChunk),
}

impl AgentCompletionIds for TaskChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> + Send {
        // Enum dispatch: each variant's own impl returns its own concrete
        // iterator type. We type-erase via Box<dyn ...> to unify them.
        let iter: Box<dyn Iterator<Item = &str> + Send + '_> = match self {
            TaskChunk::FunctionExecution(c) => {
                Box::new(c.agent_completion_ids())
            }
            TaskChunk::VectorCompletion(c) => {
                Box::new(c.agent_completion_ids())
            }
        };
        iter
    }
}

impl TaskChunk {
    pub fn vector_completion_tasks(
        &self,
    ) -> impl Iterator<Item = &super::VectorCompletionTaskChunk> {
        enum Iter<'a> {
            FunctionExecution(
                Box<
                    dyn Iterator<Item = &'a super::VectorCompletionTaskChunk>
                        + 'a,
                >,
            ),
            VectorCompletion(Option<&'a super::VectorCompletionTaskChunk>),
        }
        impl<'a> Iterator for Iter<'a> {
            type Item = &'a super::VectorCompletionTaskChunk;
            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Iter::FunctionExecution(iter) => iter.next(),
                    Iter::VectorCompletion(opt) => opt.take(),
                }
            }
        }
        match self {
            TaskChunk::FunctionExecution(function) => Iter::FunctionExecution(
                Box::new(function.inner.vector_completion_tasks()),
            ),
            TaskChunk::VectorCompletion(vector_completion) => {
                Iter::VectorCompletion(Some(&vector_completion))
            }
        }
    }

    pub fn index(&self) -> u64 {
        match self {
            TaskChunk::FunctionExecution(chunk) => chunk.index,
            TaskChunk::VectorCompletion(chunk) => chunk.index,
        }
    }

    pub fn push(&mut self, other: &TaskChunk) {
        match (self, other) {
            (
                TaskChunk::FunctionExecution(self_chunk),
                TaskChunk::FunctionExecution(other_chunk),
            ) => {
                self_chunk.push(other_chunk);
            }
            (
                TaskChunk::VectorCompletion(self_chunk),
                TaskChunk::VectorCompletion(other_chunk),
            ) => {
                self_chunk.push(other_chunk);
            }
            _ => {}
        }
    }

}
