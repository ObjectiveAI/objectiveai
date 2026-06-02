//! Free-function port of `AssistantResponseChunk::produce_files`.

use objectiveai_sdk::agent::completions::response::streaming::{
    AssistantResponseChunk, AssistantResponseChunkLog,
};

use objectiveai_sdk::logs::LogReference;

use crate::filesystem::logs::LogFile;

/// Produce log files for an `AssistantResponseChunk`. Returns
/// `(reference, files)` where `reference` points to the message file
/// and `files` contains the message itself, logprobs, and extracted
/// media.
pub fn produce_files(
    c: &AssistantResponseChunk,
    id: &str,
    route_base: &str,
) -> (LogReference, Vec<LogFile>) {
    let mut files = Vec::new();

    // All assistant-only extracts live under the kind subdir so every
    // reference from the parent assistant message log file points
    // strictly inside its own directory subtree — see the
    // nested-sub-folder rule on `LogReference`.

    // Extract logprobs to a separate file (if present).
    let logprobs_ref = c.logprobs.as_ref().map(|logprobs| {
        let logprobs_file = LogFile {
            route: format!("{route_base}/messages/assistant/logprobs"),
            id: id.to_string(),
            message_index: Some(c.index),
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(logprobs).unwrap(),
        };
        let r = LogReference::new(logprobs_file.path());
        files.push(logprobs_file);
        r
    });

    // Extract reasoning to its own file (if present). Raw text;
    // no JSON quoting/escaping — match what `read_text` /
    // `subscribe_text` consume on the read side.
    let reasoning_ref = c.reasoning.as_ref().map(|reasoning| {
        let f = LogFile {
            route: format!("{route_base}/messages/assistant/reasoning"),
            id: id.to_string(),
            message_index: Some(c.index),
            media_index: None,
            extension: "txt".to_string(),
            content: reasoning.clone().into_bytes(),
        };
        let r = LogReference::new(f.path());
        files.push(f);
        r
    });

    // Extract refusal to its own file (if present). Raw text; see
    // reasoning above.
    let refusal_ref = c.refusal.as_ref().map(|refusal| {
        let f = LogFile {
            route: format!("{route_base}/messages/assistant/refusal"),
            id: id.to_string(),
            message_index: Some(c.index),
            media_index: None,
            extension: "txt".to_string(),
            content: refusal.clone().into_bytes(),
        };
        let r = LogReference::new(f.path());
        files.push(f);
        r
    });

    // Extract each tool_call to its own file (if present).
    let tool_call_refs = c.tool_calls.as_ref().map(|tcs| {
        tcs.iter()
            .map(|tc| {
                let f = LogFile {
                    route: format!(
                        "{route_base}/messages/assistant/tool_calls"
                    ),
                    id: id.to_string(),
                    message_index: Some(c.index),
                    media_index: Some(tc.index),
                    extension: "json".to_string(),
                    content: serde_json::to_vec_pretty(tc).unwrap(),
                };
                let r = LogReference::new(f.path());
                files.push(f);
                r
            })
            .collect::<Vec<_>>()
    });

    // Extract media from content (if present). Routed under the kind-
    // specific subdir so the (response_id, index) stems don't collide
    // with a tool message at the same index.
    let content_log = c.content.clone().map(|mut content| {
        content.prepare();
        let (content_log, media_files) =
            crate::logs::agents::completions::message::rich_content::extract_media(
                content,
                &format!("{route_base}/messages/assistant"),
                id,
                c.index,
            );
        files.extend(media_files);
        content_log
    });

    let log = AssistantResponseChunkLog {
        role: c.role,
        index: c.index,
        created: c.created,
        agent: c.agent.clone(),
        model: c.model.clone(),
        upstream_id: c.upstream_id.clone(),
        reasoning: reasoning_ref,
        tool_calls: tool_call_refs,
        content: content_log,
        refusal: refusal_ref,
        finish_reason: c.finish_reason.clone(),
        logprobs: logprobs_ref,
        service_tier: c.service_tier.clone(),
        system_fingerprint: c.system_fingerprint.clone(),
        provider: c.provider.clone(),
        usage: c.usage.clone(),
    };

    let msg_file = LogFile {
        // Kind-specific subdir so this file can't collide with a tool
        // message at the same (response_id, index) — see
        // `MessageKind::file_path` for the reader-side mirror.
        route: format!("{route_base}/messages/assistant"),
        id: id.to_string(),
        message_index: Some(c.index),
        media_index: None,
        extension: "json".to_string(),
        content: serde_json::to_vec_pretty(&log).unwrap(),
    };
    let reference = LogReference::new(msg_file.path());
    files.push(msg_file);

    (reference, files)
}
