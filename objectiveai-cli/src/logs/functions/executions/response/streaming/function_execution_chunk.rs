//! Free-function ports of `FunctionExecutionChunk::produce_files` and
//! `FunctionExecutionChunk::produce_message_rows`.

use objectiveai_sdk::functions::executions::response::streaming::{
    FunctionExecutionChunk, FunctionExecutionChunkLog, task_log_reference,
};

use objectiveai_sdk::logs::LogReference;

use crate::filesystem::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

/// Produce the [`LogFile`]s for a function execution chunk. Returns
/// `None` if the chunk has no ID yet. All paths relative to `logs/`.
pub fn produce_files(
    c: &FunctionExecutionChunk,
) -> Option<(LogReference, Vec<LogFile>)> {
    const ROUTE: &str = "functions/executions/response";

    let id = &c.id;
    if id.is_empty() {
        return None;
    }

    let mut files: Vec<LogFile> = Vec::new();
    let mut task_refs: Vec<task_log_reference::LogReference> = Vec::new();

    for task in &c.tasks {
        let (reference, task_files) = super::task_chunk::produce_files(task);
        task_refs.push(reference);
        files.extend(task_files);
    }

    let reasoning_ref = c.reasoning.as_ref().map(|r| {
        let (reference, reasoning_files) =
            super::reasoning_summary_chunk::produce_files(r);
        files.extend(reasoning_files);
        reference
    });

    let retry_token_ref = c.retry_token.as_ref().map(|retry_token| {
        let rt_file = LogFile {
            route: format!("{ROUTE}/retry_token"),
            id: id.clone(),
            message_index: None,
            media_index: None,
            extension: "txt".to_string(),
            content: retry_token.clone().into_bytes(),
        };
        let r = LogReference::new(rt_file.path());
        files.push(rt_file);
        r
    });

    let log = FunctionExecutionChunkLog {
        id: c.id.clone(),
        tasks: task_refs,
        tasks_errors: c.tasks_errors,
        output: c.output.clone(),
        error: c.error.clone(),
        retry_token: retry_token_ref,
        created: c.created,
        function: c.function.clone(),
        profile: c.profile.clone(),
        object: c.object,
        usage: c.usage.clone(),
        reasoning: reasoning_ref,
    };

    let root_file = LogFile {
        route: ROUTE.to_string(),
        id: id.clone(),
        message_index: None,
        media_index: None,
        extension: "json".to_string(),
        content: serde_json::to_vec_pretty(&log).unwrap(),
    };
    let reference = LogReference::new(root_file.path());
    files.push(root_file);

    Some((reference, files))
}

/// Flat-maps message rows from every task (mirrors
/// `agent_completion_ids()`'s traversal). Reasoning summary rows are
/// also included via the reasoning chunk's delegation. Lazy and
/// `Box<dyn Iterator>`-erased at this boundary because tasks and
/// reasoning have different concrete iterator types.
pub fn produce_message_rows(
    c: &FunctionExecutionChunk,
) -> Box<dyn Iterator<Item = MessageRow> + Send + '_> {
    let task_rows = c
        .tasks
        .iter()
        .flat_map(|t| super::task_chunk::produce_message_rows(t));
    let reasoning_rows = c
        .reasoning
        .iter()
        .flat_map(|r| super::reasoning_summary_chunk::produce_message_rows(r));
    Box::new(task_rows.chain(reasoning_rows))
}
