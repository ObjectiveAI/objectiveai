//! Free-function port of `FunctionExecutionTaskChunk::produce_files`.

use objectiveai_sdk::functions::executions::response::streaming::{
    FunctionExecutionTaskChunk, function_execution_task_log_reference,
};

use crate::filesystem::logs::LogFile;

/// Produce log files for a nested function execution task. Returns
/// `(reference, files)` where `reference` carries `index`, `task_index`,
/// `task_path`, and optionally `swiss_pool_index`, `swiss_round`,
/// `split_index`. Files under `functions/executions/`.
pub fn produce_files(
    c: &FunctionExecutionTaskChunk,
) -> (function_execution_task_log_reference::LogReference, Vec<LogFile>) {
    let (path, files) = match super::function_execution_chunk::produce_files(&c.inner) {
        Some((inner_ref, files)) => (inner_ref.path, files),
        None => (String::new(), Vec::new()),
    };
    let mut reference = function_execution_task_log_reference::LogReference::new(
        path,
        c.index,
        c.task_index,
        c.task_path.clone(),
    );
    reference.swiss_pool_index = c.swiss_pool_index;
    reference.swiss_round = c.swiss_round;
    reference.split_index = c.split_index;
    (reference, files)
}
