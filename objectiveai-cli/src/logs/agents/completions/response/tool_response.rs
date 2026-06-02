//! Free-function port of `ToolResponse::produce_files`.

use objectiveai_sdk::agent::completions::response::{ToolResponse, ToolResponseLog};

use objectiveai_sdk::logs::LogReference;

use crate::filesystem::logs::LogFile;

/// Produce log files for a `ToolResponse`. Returns `(reference, files)`
/// where `reference` points to the message file and `files` contains
/// the message itself plus extracted media.
pub fn produce_files(
    tr: &ToolResponse,
    id: &str,
    route_base: &str,
) -> (LogReference, Vec<LogFile>) {
    let mut files = Vec::new();

    // Extract media from content (flattened on disk via the wire
    // chunk's `serde(flatten)` on `inner`). Routed under the kind-
    // specific subdir so the (response_id, index) stems don't collide
    // with an assistant message at the same index.
    let mut content = tr.inner.content.clone();
    content.prepare();
    let (content_log, media_files) =
        crate::logs::agents::completions::message::rich_content::extract_media(
            content,
            &format!("{route_base}/messages/tool"),
            id,
            tr.index,
        );
    files.extend(media_files);

    let log = ToolResponseLog {
        role: tr.role,
        index: tr.index,
        content: content_log,
        tool_call_id: tr.inner.tool_call_id.clone(),
        metadata: tr.inner.metadata.clone(),
    };

    let msg_file = LogFile {
        // Kind-specific subdir so this file can't collide with an
        // assistant message at the same (response_id, index) — see
        // `MessageKind::file_path` for the reader-side mirror.
        route: format!("{route_base}/messages/tool"),
        id: id.to_string(),
        message_index: Some(tr.index),
        media_index: None,
        extension: "json".to_string(),
        content: serde_json::to_vec_pretty(&log).unwrap(),
    };
    let reference = LogReference::new(msg_file.path());
    files.push(msg_file);

    (reference, files)
}
