//! Free-function ports of `VectorCompletionTaskChunk::produce_files`
//! and `VectorCompletionTaskChunk::produce_message_rows`.

use objectiveai_sdk::functions::executions::response::streaming::{
    VectorCompletionTaskChunk, vector_completion_task_log_reference,
};

use crate::filesystem::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

/// Produce log files for a vector completion task. Returns
/// `(reference, files)` where `reference` carries `index`,
/// `task_index`, `task_path`, and optionally `error`. Files under
/// `vector/completions/`.
pub fn produce_files(
    c: &VectorCompletionTaskChunk,
) -> (vector_completion_task_log_reference::LogReference, Vec<LogFile>) {
    let (path, files) =
        match crate::logs::vector::completions::response::streaming::vector_completion_chunk::produce_files(&c.inner) {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
    let mut reference = vector_completion_task_log_reference::LogReference::new(
        path,
        c.index,
        c.task_index,
        c.task_path.clone(),
    );
    if let Some(error) = &c.error {
        reference.error = Some(serde_json::to_value(error).unwrap());
    }
    (reference, files)
}

/// Delegates to the inner vector completion.
pub fn produce_message_rows(
    c: &VectorCompletionTaskChunk,
) -> impl Iterator<Item = MessageRow> + Send + '_ {
    crate::logs::vector::completions::response::streaming::vector_completion_chunk::produce_message_rows(&c.inner)
}
