use crate::agent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for creating a laboratory execution.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "laboratories.executions.request.LaboratoryExecutionCreateParams"
)]
pub struct LaboratoryExecutionCreateParams {
    /// Docker image to use for builder containers.
    pub docker_image: String,
    /// Builder agents — at least one required.
    pub builder_agents:
        Vec<agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional>,
    /// Evaluation agent for evaluating builder outputs. If not provided,
    /// no evaluation is performed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub evaluation_agent:
        Option<agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional>,

    /// Messages for builder agents.
    pub builder_messages: Vec<agent::completions::message::Message>,
    /// Messages for the evaluation agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub evaluation_messages: Option<Vec<agent::completions::message::Message>>,

    /// Output schema for evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub evaluation_output_schema:
        Option<crate::functions::expression::InputSchema>,

    /// Continuation from a previous builder completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub builder_continuation: Option<String>,
    /// Continuation from a previous evaluation completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub evaluation_continuation: Option<String>,

    /// Maximum number of evaluation retries if validation fails.
    /// Defaults to 3 if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub max_evaluation_retries: Option<u32>,

    /// Whether to keep containers after execution. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub persist: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
}
