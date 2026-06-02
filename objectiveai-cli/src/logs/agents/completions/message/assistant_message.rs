//! Free-function port of `AssistantMessage::extract`.

use objectiveai_sdk::agent::completions::message::{
    AssistantMessage, AssistantMessageLog,
};

use objectiveai_sdk::logs::LogReference;

use crate::filesystem::logs::LogFile;

/// Extract an `AssistantMessage`'s fields into per-field log files,
/// returning an [`AssistantMessageLog`] with `content` / `reasoning` /
/// `refusal` / `tool_calls` swapped for [`LogReference`]s pointing at
/// the files this function writes. Only `name` stays inline. Mirrors
/// the response-side `AssistantResponseChunk::produce_files`.
pub fn extract(
    msg: AssistantMessage,
    route_base: &str,
    id: &str,
    message_index: u64,
) -> (AssistantMessageLog, Vec<LogFile>) {
    let mut files = Vec::new();

    let reasoning_ref = msg.reasoning.map(|reasoning| {
        let f = LogFile {
            route: format!("{route_base}/messages/reasoning"),
            id: id.to_string(),
            message_index: Some(message_index),
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&reasoning).unwrap(),
        };
        let r = LogReference::new(f.path());
        files.push(f);
        r
    });

    let refusal_ref = msg.refusal.map(|refusal| {
        let f = LogFile {
            route: format!("{route_base}/messages/refusal"),
            id: id.to_string(),
            message_index: Some(message_index),
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&refusal).unwrap(),
        };
        let r = LogReference::new(f.path());
        files.push(f);
        r
    });

    let tool_call_refs = msg.tool_calls.map(|tcs| {
        tcs.into_iter()
            .enumerate()
            .map(|(tc_idx, tc)| {
                let f = LogFile {
                    route: format!("{route_base}/messages/tool_calls"),
                    id: id.to_string(),
                    message_index: Some(message_index),
                    media_index: Some(tc_idx as u64),
                    extension: "json".to_string(),
                    content: serde_json::to_vec_pretty(&tc).unwrap(),
                };
                let r = LogReference::new(f.path());
                files.push(f);
                r
            })
            .collect::<Vec<_>>()
    });

    let content_log = msg.content.map(|content| {
        let (log, content_files) = super::rich_content::extract_media(
            content,
            &format!("{route_base}/messages"),
            id,
            message_index,
        );
        files.extend(content_files);
        log
    });

    (
        AssistantMessageLog {
            content: content_log,
            name: msg.name,
            refusal: refusal_ref,
            tool_calls: tool_call_refs,
            reasoning: reasoning_ref,
        },
        files,
    )
}
