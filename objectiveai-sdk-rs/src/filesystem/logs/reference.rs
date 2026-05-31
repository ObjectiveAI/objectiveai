//! `LogReference` — the plain on-disk pointer shape every produced
//! log file uses to reference a single child file.
//!
//! On disk:
//!
//! ```json
//! { "type": "reference", "path": "agents/completions/response/messages/assistant/acc-1_0.json" }
//! ```
//!
//! For references that carry additional per-context metadata (an
//! `index`, a `task_path`, an inline `error` or `output`, etc.),
//! each chunk that needs them defines its own `LogReference` struct
//! in a sibling `*_log_reference.rs` file — same name (`LogReference`),
//! different module path. See:
//!
//! - [`super::indexed_reference::LogReference`] — `{type, path, index}`
//! - `laboratories::executions::response::streaming::builder_log_reference::LogReference`
//! - `laboratories::executions::response::streaming::evaluation_log_reference::LogReference`
//! - `functions::executions::response::streaming::reasoning_summary_log_reference::LogReference`
//! - `functions::executions::response::streaming::function_execution_task_log_reference::LogReference`
//! - `functions::executions::response::streaming::vector_completion_task_log_reference::LogReference`
//! - `functions::executions::response::streaming::task_log_reference::LogReference` (untagged enum dispatch)
//!
//! # Nested-sub-folder rule (absolute)
//!
//! Every `LogReference` (and every `*_log_reference::LogReference`
//! variant) inside a log file MUST point at a path that lives inside
//! a sub-folder of the directory containing the referencing file.
//! Sibling references (same directory) and uncle references (a
//! different sub-tree) are disallowed — they make `(response_id,
//! path)` ambiguous across kinds (assistant vs. tool), let two
//! writers race for the same on-disk filename, and break the
//! "delete the parent dir, lose everything it owns" cleanup model.
//!
//! Two carve-outs, each documented at the call site:
//!
//! 1. **Cross-endpoint chunk reuse.** A `FunctionExecutionChunk` log
//!    references its inner reasoning-summary `AgentCompletionChunk`
//!    log under `agents/completions/response/{inner_id}.json`,
//!    not under `functions/executions/response/...` — the same is
//!    true for vector-completion → agent-completion,
//!    function-invention → agent-completion, and
//!    function-invention-recursive → function-invention. These
//!    inner chunks are *the same log* whether you read them via
//!    their own endpoint or via the outer one; re-rooting them
//!    would shatter that identity. The outer log lives in its
//!    own sub-tree; the cross-endpoint reference is sideways by
//!    design.
//!
//! 2. **Notifications.** Notification files live under
//!    `agents/completions/request/notifications/...` and are
//!    keyed by the target agent-completion's `response_id` (see
//!    `AgentCompletionNotifyParams::response_id`), not by the
//!    referencing request log's id — they're delivered to a
//!    long-running completion's WebSocket stream, not produced by
//!    the request that names them.
//!
//! Everything else nests: `messages/{assistant,tool}/{response_id}_{idx}.json`,
//! `messages/{kind}/{logprobs,reasoning,refusal,tool_calls,text,
//! image,audio,video,file}/...`, `continuation/{id}.json`,
//! `response_format/{id}.json`, `retry_token/{id}.json`, etc. —
//! the parent log file's `LogReference`s only ever cross *deeper*
//! into its own subtree.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Plain on-disk pointer (`type` + `path` only).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.logs.LogReference")]
pub struct LogReference {
    #[serde(rename = "type")]
    pub r#type: LogReferenceTag,
    /// Relative on-disk path of the referenced file (under
    /// `${config_base_dir}/logs/`). Skipped when empty — the
    /// no-data sentinel case used by some wrappers when the inner
    /// chunk has no content to log.
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

/// Constant `"reference"` discriminator — the `"type"` field on
/// every `LogReference` variant.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "filesystem.logs.LogReferenceTag")]
pub enum LogReferenceTag {
    Reference,
}
