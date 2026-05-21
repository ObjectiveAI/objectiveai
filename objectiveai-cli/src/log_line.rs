//! Streaming "log file is ready" handshake between the streaming `create`
//! commands and the `--detach` parent process. The parent watches the
//! child's stdout for a [`LogStreamReady`] JSONL notification and exits
//! cleanly once it sees one.

use objectiveai_sdk::cli::output::{Cleared, Handle, Items, LogContent, LogStreamReady, Output};

/// Emit the log-stream-ready notification with the given log id.
pub async fn emit_log_stream_ready(id: &str, handle: &Handle) {
    Output::<LogStreamReady>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: LogStreamReady {
        log_stream_ready: id.to_string(),
    } })
    .emit(handle)
    .await;
}

/// Returns the log id if `line` is a log-stream-ready notification.
pub fn parse_log_stream_ready(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let parsed: Output<LogStreamReady> = serde_json::from_str(trimmed).ok()?;
    match parsed {
        Output::Notification(objectiveai_sdk::cli::output::Notification {
            value: LogStreamReady { log_stream_ready },
            ..
        }) => Some(log_stream_ready),
        Output::Error(_) | Output::Begin | Output::End => None,
    }
}

/// Translate the upstream `LogContent` (which has no serde derives)
/// into the cli-lib wire shape and emit.
pub async fn emit_log_content(content: objectiveai_sdk::filesystem::logs::LogContent, handle: &Handle) {
    let wire = match content {
        objectiveai_sdk::filesystem::logs::LogContent::Json(v) => LogContent::Json { content: v },
        objectiveai_sdk::filesystem::logs::LogContent::DataUrl(s) => LogContent::DataUrl {
            content_data_url: s,
        },
    };
    Output::<LogContent>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: wire }).emit(handle).await;
}

/// Emit a list of log directory entries as `Items<LogListItem>`.
pub async fn emit_log_list(
    items: Vec<objectiveai_sdk::filesystem::logs::ListItem>,
    handle: &Handle,
) {
    Output::<Items<objectiveai_sdk::filesystem::logs::ListItem>>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: Items { items } })
        .emit(handle)
        .await;
}

/// Emit the count of cleared log files as `Cleared`.
pub async fn emit_log_clear_count(count: u64, handle: &Handle) {
    Output::<Cleared>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: Cleared { cleared: count } })
        .emit(handle)
        .await;
}
