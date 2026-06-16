use serde::{Deserialize, Serialize};

/// The `status` discriminator on a terminal `end` line, with its
/// optional `error` payload flattened into the parent object.
///
/// There is no `cancelled` variant: in-flight cancellation isn't
/// supported (see `StdioInput`). Anything that *would* have been a
/// cancel surfaces as an `error` with a descriptive message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StdioEndStatus {
    /// `{"status":"ok"}`
    Ok,
    /// `{"status":"error","error":"<msg>"}`
    Error { error: String },
}
