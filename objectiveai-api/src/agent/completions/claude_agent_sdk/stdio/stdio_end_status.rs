use serde::{Deserialize, Serialize};

/// The `status` discriminator on a terminal `end` line, with its
/// optional `error` payload flattened into the parent object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StdioEndStatus {
    /// `{"status":"ok"}`
    Ok,
    /// `{"status":"cancelled"}`
    Cancelled,
    /// `{"status":"error","error":"<msg>"}`
    Error { error: String },
}
