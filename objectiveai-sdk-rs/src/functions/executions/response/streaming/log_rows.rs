//! Streaming-content `log_rows` walker for function-execution chunks.
//!
//! Recurses into every nested task — function-execution tasks chain
//! back into this same walker; vector-completion tasks forward to
//! [`VectorCompletionChunk::log_rows`]. The reasoning summary slot
//! holds an embedded agent completion whose `log_rows()` flows
//! through as well. Every emitted row is keyed by its
//! globally-unique response id, so the writer's shared UPSERT path
//! doesn't need to know which enclosing chunk produced it.

use crate::logs::{LogRowIter, LogValue};

use super::{FunctionExecutionChunk, TaskChunk};

impl FunctionExecutionChunk {
    pub fn log_rows<'a>(&'a self) -> LogRowIter<'a> {
        let task_iter = self
            .tasks
            .iter()
            .flat_map(|task| task_log_rows(task));
        let reasoning_iter = self
            .reasoning
            .iter()
            .flat_map(|r| r.inner.log_rows());
        Box::new(task_iter.chain(reasoning_iter))
    }
}

fn task_log_rows<'a>(
    task: &'a TaskChunk,
) -> Box<dyn Iterator<Item = LogValue<'a>> + Send + 'a> {
    match task {
        TaskChunk::FunctionExecution(wrapper) => wrapper.inner.log_rows(),
        TaskChunk::VectorCompletion(wrapper) => wrapper.inner.log_rows(),
    }
}
