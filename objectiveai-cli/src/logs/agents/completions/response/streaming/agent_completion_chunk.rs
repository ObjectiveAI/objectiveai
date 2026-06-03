//! Free-function ports of `AgentCompletionChunk::produce_files` and
//! `AgentCompletionChunk::produce_message_rows`.

use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AgentCompletionChunkLog, MessageChunk,
};

use objectiveai_sdk::logs::LogReference;

use objectiveai_sdk::cli::command::agents::read::subscribe::RequestMessageKind;

use crate::filesystem::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

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
        agent_instance_hierarchy: c.agent_instance_hierarchy.clone(),
        agent_id: c.agent_id.clone(),
        agent_full_id: c.agent_full_id.clone(),
        agent_remote: c.agent_remote.clone(),
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

/// Yield one [`MessageRow`] per `MessageChunk` for the SQLite
/// `messages` table. Lazy: borrows from `c`, never collects.
///
/// `agent_instance_hierarchy` is this chunk's `id`; `path` points at
/// the per-message log file under `agents/completions/response/messages/`.
/// Returns an empty iterator when `id` is empty (the chunk hasn't been
/// assigned a response id yet — same gate `produce_files` uses).
pub fn produce_message_rows(
    c: &AgentCompletionChunk,
) -> impl Iterator<Item = MessageRow> + Send + '_ {
    let id = c.id.as_str();
    let created = c.created;
    let empty = c.id.is_empty();
    c.messages.iter().filter_map(move |m| {
        if empty {
            return None;
        }
        let kind = match m {
            MessageChunk::Assistant(_) => RequestMessageKind::AssistantResponse,
            MessageChunk::Tool(_) => RequestMessageKind::ToolResponse,
        };
        let idx = m.index();
        Some(MessageRow {
            agent_instance_hierarchy: id.to_string(),
            // Same value as agent_instance_hierarchy at this stage — the writer
            // will lineage-stamp `agent_instance_hierarchy` but `response_id`
            // stays bare so the reader doesn't have to parse it
            // back out of a stamped string.
            response_id: id.to_string(),
            kind,
            index: idx,
            // Bare id — the route is reconstructed from
            // (kind, response_id, path) by `RequestMessageKind::file_path`.
            path: format!("{idx}"),
            timestamp: created,
        })
    })
}
