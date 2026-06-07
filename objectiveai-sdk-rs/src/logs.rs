//! SDK-side `logs` module.
//!
//! Hosts the shared ref shapes every `*Log` struct uses to point at a
//! row in another postgres table:
//!
//! - [`LogRef`] — `{ table, id }`. Single ref into the named table.
//! - [`RichContentLogRef`] — untagged enum of `Solo(LogRef)` or
//!   `Many(Vec<LogRef>)`. Used for content slots that may carry either
//!   a single content row or an ordered list of mixed media rows.
//! - [`LogTable`] — closed enum of every postgres table a ref can
//!   point into: five content-addressed leaf tables (`text`, `image`,
//!   `audio`, `video`, `file`) plus six request/response tables.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `{ table, id }` ref. Every `*Log` content slot whose underlying
/// value is a plain string (refusal, tool-call arguments, continuation
/// tokens, function input JSON) — and every cross-completion ref
/// (per-agent vector slot, per-task function slot, reasoning summary)
/// — is shaped this way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "LogRef")]
pub struct LogRef {
    pub table: LogTable,
    pub id: i64,
}

impl LogRef {
    pub fn new(table: LogTable, id: i64) -> Self {
        Self { table, id }
    }
}

/// Ref shape for any content slot whose wire form is a `RichContent`
/// (i.e. content fields on user / system / developer / assistant /
/// tool messages, and tool-response content).
///
/// Wire form is untagged — the writer emits `Solo` when the source was
/// a plain string (lowered to one `LogRef` into the `text` table) and
/// `Many` when the source was a real rich-content list (one `LogRef`
/// per part, table-tagged by media kind).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "RichContentLogRef")]
pub enum RichContentLogRef {
    #[schemars(title = "Solo")]
    Solo(LogRef),
    #[schemars(title = "Many")]
    Many(Vec<LogRef>),
}

/// Closed enum of every postgres table a [`LogRef`] can point into.
///
/// Content-addressed leaf tables (deduplicated payloads — same body
/// inserted twice maps to the same row id):
/// - `text` — plain string content (refusal, reasoning, tool-call
///   arguments, continuation/retry tokens, any RichContent::Text
///   part).
/// - `image`, `audio`, `video`, `file` — JSONB bodies of the matching
///   media struct (`ImageUrl`, `InputAudio`, `VideoUrl`, `File`).
/// - `input` — JSONB body for function-execution inputs (structured
///   data, kept as its own table since it isn't string-shaped).
///
/// Six request/response tables hold the stripped log envelopes
/// themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "LogTable")]
pub enum LogTable {
    Text,
    Image,
    Audio,
    Video,
    File,
    Input,
    AgentCompletionRequest,
    AgentCompletionResponse,
    VectorCompletionRequest,
    VectorCompletionResponse,
    FunctionExecutionRequest,
    FunctionExecutionResponse,
}

impl LogTable {
    /// Fully-qualified table name in postgres, including the `logs.`
    /// schema prefix. Used by the CLI writer when constructing
    /// `INSERT INTO …` statements.
    pub fn fq_name(self) -> &'static str {
        match self {
            LogTable::Text => "logs.text",
            LogTable::Image => "logs.image",
            LogTable::Audio => "logs.audio",
            LogTable::Video => "logs.video",
            LogTable::File => "logs.file",
            LogTable::Input => "logs.input",
            LogTable::AgentCompletionRequest => "logs.agent_completion_requests",
            LogTable::AgentCompletionResponse => "logs.agent_completion_responses",
            LogTable::VectorCompletionRequest => "logs.vector_completion_requests",
            LogTable::VectorCompletionResponse => "logs.vector_completion_responses",
            LogTable::FunctionExecutionRequest => "logs.function_execution_requests",
            LogTable::FunctionExecutionResponse => "logs.function_execution_responses",
        }
    }
}
