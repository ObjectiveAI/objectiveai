use serde::{Deserialize, Serialize};

use super::StdioDiagLevel;

/// One line emitted by the runner on stderr.
///
/// Two variants:
///
/// - [`StdioError::Diag`] is per-request and carries `id`. It's the
///   normal channel for non-fatal warnings. The `id` is always the
///   second field on the wire.
/// - [`StdioError::Fatal`] is process-level and carries no `id`. The
///   runner only emits one of these on its way to a non-zero exit
///   (import failure, asyncio init crash, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StdioError {
    /// Per-request diagnostic, tagged with `id`.
    Diag {
        id: String,
        level: StdioDiagLevel,
        message: String,
    },
    /// Process-level fatal — runner is exiting non-zero. Untagged.
    Fatal { message: String },
}
