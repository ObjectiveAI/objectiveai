//! Free-function port of `TaskChunk::produce_files` (dispatch).

use objectiveai_sdk::functions::executions::response::streaming::{
    TaskChunk, task_log_reference,
};

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
