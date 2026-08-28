//! The `result` records: how a turn ended.

use serde::{Deserialize, Serialize};

use super::system::FastModeState;

/// A `type: "result"` record, discriminated by `subtype`: the turn's
/// verdict and its bill.
///
/// # It is not always the last word
///
/// A result can be HELD BACK while background agents still run, then
/// flushed after later records; and even an unheld one is followed by
/// `session_state_changed`, late task notifications, and prompt
/// suggestions. The authoritative turn-over signal is
/// [`session_state_changed { state: idle }`](super::System::SessionStateChanged),
/// not this record's arrival.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subtype")]
pub enum Result {
    /// The turn finished.
    #[serde(rename = "success")]
    Success {
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        structured_output: Option<serde_json::Value>,
        /// Fast mode's state, when the feature is in play.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fast_mode_state: Option<FastModeState>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// The turn died mid-flight.
    #[serde(rename = "error_during_execution")]
    ErrorDuringExecution(ResultError),
    /// The turn hit the turn cap.
    #[serde(rename = "error_max_turns")]
    ErrorMaxTurns(ResultError),
    /// The turn hit the budget cap.
    #[serde(rename = "error_max_budget_usd")]
    ErrorMaxBudgetUsd(ResultError),
    /// The turn never produced valid structured output.
    #[serde(rename = "error_max_structured_output_retries")]
    ErrorMaxStructuredOutputRetries(ResultError),
}

/// What every error result carries: the success fields minus the
/// answer, plus the reasons.
///
/// One shape for the four subtypes because the source gives them one
/// schema; which cap was hit is the tag's knowledge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultError {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast_mode_state: Option<FastModeState>,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The summed usage a result carries.
///
/// The schema says `unknown`; the shape here is the source's own
/// zero-value (`EMPTY_USAGE`), which is the API's usage widened with
/// Claude Code's additions. The four token counts are always
/// present; everything past them defaults, because a shape the
/// schema does not promise is a shape a reader should not demand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Input tokens billed, not counting cache reads or writes.
    pub input_tokens: u64,
    /// Tokens written to the prompt cache.
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    /// Tokens read from the prompt cache.
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    /// Output tokens billed.
    pub output_tokens: u64,
    /// Server-side tool spend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_tool_use: Option<ServerToolUse>,
    /// The service tier that served it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// Cache writes, by lifetime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<CacheCreation>,
    /// Where inference ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
    /// Per-iteration detail, shape unpromised — the clone types it
    /// in a file it does not carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iterations: Option<Vec<serde_json::Value>>,
    /// The speed tier that served it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
}

/// Server-side tool counts inside [`Usage`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
pub struct ServerToolUse {
    /// Web searches run.
    #[serde(default)]
    pub web_search_requests: u64,
    /// Web fetches run.
    #[serde(default)]
    pub web_fetch_requests: u64,
}

/// Cache writes by lifetime inside [`Usage`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
pub struct CacheCreation {
    /// Tokens cached for an hour.
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u64,
    /// Tokens cached for five minutes.
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u64,
}

/// One model's share of a result's usage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionDenial {
    /// The tool.
    pub tool_name: String,
    /// The call.
    pub tool_use_id: String,
    /// What it was called with.
    pub tool_input: indexmap::IndexMap<String, serde_json::Value>,
}
