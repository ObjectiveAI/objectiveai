use crate::{agent, functions};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.request.FunctionInventionCreateParams")]
pub struct FunctionInventionCreateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub remote: Option<crate::Remote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub overwrite: Option<bool>,
    pub state: functions::inventions::ParamsStateOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    pub agent: agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub prompt: functions::inventions::prompts::InlinePromptOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// Maximum number of retries per invention step.
    /// Each step is one agent completion (which itself may loop internally
    /// via tool calls). If the step's validation still fails after the
    /// agent loop ends, the step is retried up to this many times.
    /// Defaults to 3 if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub max_step_retries: Option<u32>,
    /// Continuation from a previous completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
}
