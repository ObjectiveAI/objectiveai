//! Streaming "log file is ready" handshake helpers + log-content
//! emission shared between the `logs` subcommands and `detach.rs`.
//!
//! The actual `emit_log_stream_ready` lives in `objectiveai-cli-stream`
//! now; the cli only PARSES the handshake (via [`parse_log_stream_ready`])
//! when its `detach.rs` parent watches an orphan's stdout for the
//! ready-line.

use objectiveai_sdk::cli::output::{
    Cleared, Handle, Items, LogContent, LogStreamReady, Notification, NotificationValue,
    Output,
};

/// Returns the log id if `line` is a log-stream-ready notification.
pub fn parse_log_stream_ready(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let parsed: Output = serde_json::from_str(trimmed).ok()?;
    match parsed {
        Output::Notification(Notification {
            value: NotificationValue::LogStreamReady(LogStreamReady { log_stream_ready }),
            ..
        }) => Some(log_stream_ready),
        _ => None,
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
    Output::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: (wire).into() }).emit(handle).await;
}

/// Emit a list of log directory entries as `Items<LogListItem>`.
pub async fn emit_log_list(
    items: Vec<objectiveai_sdk::filesystem::logs::ListItem>,
    handle: &Handle,
) {
    Output::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: objectiveai_sdk::cli::output::NotificationValue::other(&(Items { items })) })
        .emit(handle)
        .await;
}

/// Emit the count of cleared log files as `Cleared`.
pub async fn emit_log_clear_count(count: u64, handle: &Handle) {
    Output::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: (Cleared { cleared: count }).into() })
        .emit(handle)
        .await;
}
