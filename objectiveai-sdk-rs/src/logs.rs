//! SDK-side `logs` module.
//!
//! Hosts the structural types that every `*Log` data type needs to
//! express its on-disk shape — primarily [`LogReference`] (`{type,
//! path}`), the indexed variant [`IndexedLogReference`], and the
//! constant `"reference"` discriminator [`LogReferenceTag`].
//!
//! These are pure data shapes. They live here in the SDK because the
//! `*Log` types embed them as fields; the filesystem behavior that
//! constructs the values (path joining, file writes, etc.) lives in
//! `objectiveai-cli` per `feedback_extract_methods_relocate_to_cli`.
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
///
/// Carries an optional `error` so that wrappers around inner streams
/// which errored before producing an ID-bearing chunk still surface
/// what went wrong at that index. Exactly one of `path` (the resolved
/// log file) or `error` (the upstream failure) will be populated in
/// practice — both `None` would mean the wrapper has nothing to say.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "IndexedLogReference")]
pub struct IndexedLogReference {
    #[serde(rename = "type")]
    pub r#type: LogReferenceTag,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub path: Option<String>,
    pub index: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<crate::error::ResponseError>,
}

impl IndexedLogReference {
    pub fn new(path: String, index: u64) -> Self {
        Self {
            r#type: LogReferenceTag::Reference,
            path: Some(path),
            index,
            error: None,
        }
    }

    /// Like [`Self::new`] but stamps an upstream error onto the ref —
    /// used by wrappers when the inner stream failed before producing
    /// an ID-bearing chunk we could write a log file for.
    pub fn with_error(
        index: u64,
        error: crate::error::ResponseError,
    ) -> Self {
        Self {
            r#type: LogReferenceTag::Reference,
            path: None,
            index,
            error: Some(error),
        }
    }
}
