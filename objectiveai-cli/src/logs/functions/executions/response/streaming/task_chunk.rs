//! Free-function ports of `TaskChunk::produce_files` and
//! `TaskChunk::produce_message_rows` (both dispatch).

use objectiveai_sdk::functions::executions::response::streaming::{
    TaskChunk, task_log_reference,
};

use crate::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

/// Produce log files for a task chunk. Returns `(reference, files)`
/// where `reference` is the untagged `task_log_reference::LogReference`
/// dispatching to whichever variant the task is.
pub fn produce_files(c: &TaskChunk) -> (task_log_reference::LogReference, Vec<LogFile>) {
    match c {
        TaskChunk::FunctionExecution(chunk) => {
            let (reference, files) =
                super::function_execution_task_chunk::produce_files(chunk);
            (
                task_log_reference::LogReference::FunctionExecution(reference),
                files,
            )
        }
        TaskChunk::VectorCompletion(chunk) => {
            let (reference, files) =
                super::vector_completion_task_chunk::produce_files(chunk);
            (
                task_log_reference::LogReference::VectorCompletion(reference),
                files,
            )
        }
    }
}

/// Delegates to whichever variant the task is. Erased to
/// `Box<dyn Iterator>` because the two variants' iterators have
/// different concrete types.
pub fn produce_message_rows(
    c: &TaskChunk,
) -> Box<dyn Iterator<Item = MessageRow> + Send + '_> {
    match c {
        TaskChunk::FunctionExecution(chunk) => Box::new(
            super::function_execution_task_chunk::produce_message_rows(chunk),
        ),
        TaskChunk::VectorCompletion(chunk) => Box::new(
            super::vector_completion_task_chunk::produce_message_rows(chunk),
        ),
    }
}
