//! `LogReference` — the plain on-disk pointer shape every produced log
//! file uses to reference a single child file. Lives in the SDK root
//! so the `*Log` data types can express on-disk shape without
//! depending on any filesystem module (which has been relocated to
//! `objectiveai-cli`).
//!
//! On disk:
//!
//! ```json
//! { "type": "reference", "path": "agents/completions/response/messages/assistant/acc-1_0.json" }
//! ```
//!
//! For references that carry additional per-context metadata (an
//! `index`, a `task_path`, an inline `error` or `output`, etc.), each
//! chunk that needs them defines its own `LogReference` struct in a
//! sibling `*_log_reference.rs` file — same name (`LogReference`),
//! different module path.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Plain on-disk pointer (`type` + `path` only).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "LogReference")]
pub struct LogReference {
    #[serde(rename = "type")]
    pub r#type: LogReferenceTag,
    /// Relative on-disk path of the referenced file (under
    /// `${config_base_dir}/logs/`). Skipped when empty — the no-data
    /// sentinel case used by some wrappers when the inner chunk has
    /// no content to log.
    #[serde(skip_serializing_if = "String::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub path: String,
}

impl LogReference {
    pub fn new(path: String) -> Self {
        Self {
            r#type: LogReferenceTag::Reference,
            path,
        }
    }
}

/// Constant `"reference"` discriminator — the `"type"` field on every
/// `LogReference` variant.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "LogReferenceTag")]
pub enum LogReferenceTag {
    Reference,
}

/// `LogReference` for log files keyed by an `index` — used by
/// per-agent / per-invention completion wrappers that need to preserve
/// their position within a parent collection (a vector completion's
/// swarm-index, an invention's per-invention index, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "IndexedLogReference")]
pub struct IndexedLogReference {
    #[serde(rename = "type")]
    pub r#type: LogReferenceTag,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub path: String,
    pub index: u64,
}

impl IndexedLogReference {
    pub fn new(path: String, index: u64) -> Self {
        Self {
            r#type: LogReferenceTag::Reference,
            path,
            index,
        }
    }
}
