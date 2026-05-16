use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Emitted mid-stream by long-running `*/create` commands once a log
/// id has been allocated and the log file is available. Replaces the
/// prior `"Logs ID: ..."` plaintext sentinel.
///
/// Wire: `{"type":"notification","log_stream_ready":"<id>"}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.LogStreamReady")]
pub struct LogStreamReady {
    pub log_stream_ready: String,
}
