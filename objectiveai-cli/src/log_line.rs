//! Streaming "log file is ready" handshake.
//!
//! Emitted by the long-running streaming `create` commands during the
//! detach flow so the parent process can detect when the child has
//! claimed a log id and become orphan-safe.
//!
//! Wire shape:
//!   `{"type":"notification","log_stream_ready":"<id>"}`

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LogStreamReady {
    pub log_stream_ready: String,
}

/// Emits the log-stream-ready notification.
pub fn emit_log_stream_ready(id: &str) {
    objectiveai_cli_lib::output::Output::<LogStreamReady>::Notification(LogStreamReady {
        log_stream_ready: id.to_string(),
    })
    .emit();
}

/// Returns the log id if the line is a log-stream-ready notification.
pub fn parse_log_stream_ready(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let parsed: objectiveai_cli_lib::output::Output<LogStreamReady> =
        serde_json::from_str(trimmed).ok()?;
    match parsed {
        objectiveai_cli_lib::output::Output::Notification(LogStreamReady { log_stream_ready }) => {
            Some(log_stream_ready)
        }
        objectiveai_cli_lib::output::Output::Error(_) => None,
    }
}
