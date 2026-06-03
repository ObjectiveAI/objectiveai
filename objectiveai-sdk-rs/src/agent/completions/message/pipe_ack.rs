//! `PipeAck` — server-side response to one `RichContent` notification
//! line written into the per-agent socket at
//! `${config_base_dir}/pipes/<agent_instance_hierarchy>/socket`.
//!
//! The cli-stream socket handler (see `objectiveai-cli-stream/src/pipes.rs`)
//! used to be write-only from the client side — clients pushed an
//! NDJSON `RichContent` and disconnected, with no visibility into
//! whether the line was actually parsed, queued, and dispatched
//! upstream. `agents message` needs that visibility so it knows
//! whether to fall back to a continuation; the server now writes
//! one `PipeAck` line after each processed input line.
//!
//! Wire shape (one NDJSON line):
//!
//! - `{"type":"ok"}`
//! - `{"type":"error","message":"<…>"}`
//!
//! Backward-compatible: old clients that close the socket immediately
//! after writing see a broken pipe on the server's ack write, which
//! the server silently swallows.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.message.PipeAck")]
pub enum PipeAck {
    /// The `RichContent` line parsed, queued in the writer task, and
    /// dispatched to the API server successfully.
    #[schemars(title = "Ok")]
    Ok,
    /// Something failed — either the line wasn't valid
    /// `RichContent` JSON or the API-side notify dispatch returned
    /// an error. `message` is human-readable and not load-bearing.
    #[schemars(title = "Error")]
    Error { message: String },
}
