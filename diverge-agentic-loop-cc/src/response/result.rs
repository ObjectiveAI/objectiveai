//! The `result` records: how a turn ended.

use diverge_sdk::provider::endpoints::containers::agents::run::server::response;
use serde::Deserialize;

use super::message::{CacheCreation, ServerToolUse};
use super::system::FastModeState;

/// A `type: "result"` record: the turn's verdict and its bill.
///
/// Two arms, which is the source's own shape — one success schema,
/// one error schema whose `subtype` is a four-value enum. Untagged,
/// with the literals as marker fields, like every union here.
///
/// # It is not always the last word
///
/// A result can be HELD BACK while background agents still run, then
/// flushed after later records; and even an unheld one is followed by
/// `session_state_changed`, late task notifications, and prompt
/// suggestions. The authoritative turn-over signal is
/// [`session_state_changed { state: idle }`](super::system::System::SessionStateChanged),
/// not this record's arrival.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Result {
    /// The turn finished.
    Success {
        /// Always `result`.
        r#type: ResultType,
        /// Always `success`.
        subtype: SuccessSubtype,
        /// Wall-clock, whole turn.
        duration_ms: f64,
        /// Wall-clock inside API calls.
        duration_api_ms: f64,
        /// Whether the final assistant message was an API failure —
        /// a "success" carries the shape, not the verdict.
        is_error: bool,
        /// How many turns ran.
        num_turns: u64,
        /// The final assistant message's text.
        result: String,
        /// Why generation stopped, as the API worded it.
        stop_reason: Option<String>,
        /// The bill.
        total_cost_usd: f64,
        /// The usage, summed.
        usage: Usage,
        /// The usage, per model.
        #[serde(rename = "modelUsage")]
        model_usage: indexmap::IndexMap<String, ModelUsage>,
        /// Every tool call a permission rule refused.
        permission_denials: Vec<PermissionDenial>,
        /// The structured output, when one was requested — whatever
        /// shape the caller's schema gave it.
        structured_output: Option<serde_json::Value>,
        /// Fast mode's state, when the feature is in play.
        fast_mode_state: Option<FastModeState>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// The turn ended badly; [`ResultError::subtype`] says how.
    Error(ResultError),
}

/// The `result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    /// The only value.
    #[default]
    Result,
}

/// The `success` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SuccessSubtype {
    /// The only value.
    #[default]
    Success,
}

/// An error result: the success fields minus the answer, plus the
/// reasons. One shape for the four subtypes because the source gives
/// them one schema; which cap was hit is
/// [`subtype`](Self::subtype)'s knowledge.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ResultError {
    /// Always `result`.
    pub r#type: ResultType,
    /// Which way the turn ended.
    pub subtype: ResultErrorSubtype,
    /// Wall-clock, whole turn.
    pub duration_ms: f64,
    /// Wall-clock inside API calls.
    pub duration_api_ms: f64,
    /// Whether the last assistant message was an API failure.
    pub is_error: bool,
    /// How many turns ran.
    pub num_turns: u64,
    /// Why generation stopped, as the API worded it.
    pub stop_reason: Option<String>,
    /// The bill.
    pub total_cost_usd: f64,
    /// The usage, summed.
    pub usage: Usage,
    /// The usage, per model.
    #[serde(rename = "modelUsage")]
    pub model_usage: indexmap::IndexMap<String, ModelUsage>,
    /// Every tool call a permission rule refused.
    pub permission_denials: Vec<PermissionDenial>,
    /// What went wrong, in words — the cap's message, then whatever
    /// the error log held.
    pub errors: Vec<String>,
    /// Fast mode's state, when the feature is in play.
    pub fast_mode_state: Option<FastModeState>,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The four ways a turn ends badly — the error schema's own
/// `subtype` enum.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ResultErrorSubtype {
    /// The turn died mid-flight.
    ErrorDuringExecution,
    /// The turn hit the turn cap.
    ErrorMaxTurns,
    /// The turn hit the budget cap.
    ErrorMaxBudgetUsd,
    /// The turn never produced valid structured output.
    ErrorMaxStructuredOutputRetries,
}

/// The summed usage a result carries.
///
/// The schema says `unknown`; the shape here is the source's own
/// zero-value (`EMPTY_USAGE`), which is the API's usage widened with
/// Claude Code's additions. Only the input and output counts are
/// demanded; everything else is an `Option`, because a shape the
/// schema does not promise is a shape a reader should not demand.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Usage {
    /// Input tokens billed, not counting cache reads or writes.
    pub input_tokens: u64,
    /// Tokens written to the prompt cache.
    pub cache_creation_input_tokens: Option<u64>,
    /// Tokens read from the prompt cache.
    pub cache_read_input_tokens: Option<u64>,
    /// Output tokens billed.
    pub output_tokens: u64,
    /// Server-side tool spend.
    pub server_tool_use: Option<ServerToolUse>,
    /// The service tier that served it.
    pub service_tier: Option<String>,
    /// Cache writes, by lifetime.
    pub cache_creation: Option<CacheCreation>,
    /// Where inference ran.
    pub inference_geo: Option<String>,
    /// Per-iteration detail, shape unpromised — the clone types it
    /// in a file it does not carry.
    pub iterations: Option<Vec<serde_json::Value>>,
    /// The speed tier that served it.
    pub speed: Option<String>,
}

/// One model's share of a result's usage.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ModelUsage {
    /// Input tokens.
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,
    /// Output tokens.
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,
    /// Cache reads.
    #[serde(rename = "cacheReadInputTokens")]
    pub cache_read_input_tokens: u64,
    /// Cache writes.
    #[serde(rename = "cacheCreationInputTokens")]
    pub cache_creation_input_tokens: u64,
    /// Web searches.
    #[serde(rename = "webSearchRequests")]
    pub web_search_requests: u64,
    /// This model's share of the bill.
    #[serde(rename = "costUSD")]
    pub cost_usd: f64,
    /// The model's context window.
    #[serde(rename = "contextWindow")]
    pub context_window: u64,
    /// The model's output ceiling.
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: u64,
}

/// One tool call a permission rule refused.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PermissionDenial {
    /// The tool.
    pub tool_name: String,
    /// The call.
    pub tool_use_id: String,
    /// What it was called with.
    pub tool_input: indexmap::IndexMap<String, serde_json::Value>,
}

impl Result {
    /// This record's chunks: the bill, on BOTH arms — an error
    /// result still spent the tokens, so it still bills (the api
    /// crate's choice, kept). The failure itself does not convert:
    /// it travels as the stream's error, yielded by the reader
    /// before this runs, and its fatality is the consumer's to
    /// decide by what follows.
    ///
    /// The verdict fields (the final text, the stop reason, the
    /// durations, the per-model breakdown) have no chunk home and
    /// say nothing here; the usage is the record's one utterance.
    pub fn into_chunks(
        self,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        match self {
            Result::Success { usage, .. } => usage.into_chunks(chunks),
            Result::Error(error) => error.usage.into_chunks(chunks),
        }
    }
}

impl ResultError {
    /// The failure as a notification's (or an HTTP error's) message
    /// body.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": self.subtype.as_str(),
            "errors": self.errors,
        })
    }

}

impl ResultErrorSubtype {
    /// The wire literal.
    pub fn as_str(self) -> &'static str {
        match self {
            ResultErrorSubtype::ErrorDuringExecution => {
                "error_during_execution"
            }
            ResultErrorSubtype::ErrorMaxTurns => "error_max_turns",
            ResultErrorSubtype::ErrorMaxBudgetUsd => {
                "error_max_budget_usd"
            }
            ResultErrorSubtype::ErrorMaxStructuredOutputRetries => {
                "error_max_structured_output_retries"
            }
        }
    }
}

impl Usage {
    /// The one usage chunk of the whole run — the api crate's
    /// doctrine: per-message usage is never read, nothing
    /// accumulates, and the `result` record's summed usage is the
    /// bill.
    ///
    /// Prompt tokens are the billed input PLUS both cache sides —
    /// writes and reads — exactly as the api crate counts them;
    /// absent cache counts count zero. The detail breakdowns, the
    /// per-model shares and the dollar cost have no home in the
    /// chunk's three flat counters and are dropped.
    pub fn into_chunks(
        self,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        let prompt_tokens = self.input_tokens
            + self.cache_creation_input_tokens.unwrap_or(0)
            + self.cache_read_input_tokens.unwrap_or(0);
        let completion_tokens = self.output_tokens;
        chunks.push(response::AgenticLoopChunk::Usage(
            response::UsageChunk {
                r#type: Default::default(),
                completion_tokens,
                prompt_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                meta: None,
            },
        ));
    }
}
