use crate::{agent, functions};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Parameters for creating a function execution.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.executions.request.FunctionExecutionCreateParams")]
pub struct FunctionExecutionCreateParams {
    /// The function to execute (inline definition or remote path).
    pub function: functions::FullInlineFunctionOrRemoteCommitOptional,
    /// The profile to use (inline definition or remote path).
    pub profile: functions::InlineProfileOrRemoteCommitOptional,

    // --- Caching and retry options ---
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub from_cache: Option<bool>,

    // --- Reasoning configuration ---
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<super::Reasoning>,

    // --- Core configuration ---
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub strategy: Option<super::Strategy>,
    pub input: functions::expression::InputValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub split: Option<bool>,
    /// If `true`, invert every output in the streamed response *after* the
    /// inner function has finished computing — scalar outputs become
    /// `1 - x`, vector outputs are reversed in place. The expression
    /// evaluator inside the function still sees the original scores; only
    /// the chunks delivered to the client (and the aggregated response
    /// passed to the usage handler) are inverted. Useful when a function
    /// is naturally written to score "lower is better" but the consumer
    /// wants "higher is better", or vice versa.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub invert: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// Continuation from a previous completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
}
