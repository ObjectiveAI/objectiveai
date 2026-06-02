//! Free-function port of `AgentCompletionChunk::produce_files`.

use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AgentCompletionChunkLog,
};

use crate::filesystem::logs::{LogFile, LogReference};

/// Produce the [`LogFile`]s for the log file structure. Returns `None`
/// if the chunk has no ID yet. All paths are relative to the `logs/`
/// root directory, under `agents/completions/`.
pub fn produce_files(
    c: &AgentCompletionChunk,
) -> Option<(LogReference, Vec<LogFile>)> {
    const ROUTE: &str = "agents/completions/response";

    let id = &c.id;
    if id.is_empty() {
        return None;
    }

    let mut files: Vec<LogFile> = Vec::new();
    let mut message_refs: Vec<LogReference> = Vec::new();

    for msg in &c.messages {
        let (reference, msg_files) =
            super::message_chunk::produce_files(msg, id, ROUTE);
        message_refs.push(reference);
        files.extend(msg_files);
    }

    // Extract continuation to a separate file (if present).
    let continuation_ref = c.continuation.as_ref().map(|continuation| {
        let cont_file = LogFile {
            route: format!("{ROUTE}/continuation"),
            id: id.clone(),
            message_index: None,
            media_index: None,
            extension: "txt".to_string(),
            content: continuation.clone().into_bytes(),
        };
        let r = LogReference::new(cont_file.path());
        files.push(cont_file);
        r
    });

    let log = AgentCompletionChunkLog {
        id: c.id.clone(),
        created: c.created,
        messages: message_refs,
        object: c.object,
        usage: c.usage.clone(),
        upstream: c.upstream,
        error: c.error.clone(),
        continuation: continuation_ref,
        messages_queued: c.messages_queued,
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
