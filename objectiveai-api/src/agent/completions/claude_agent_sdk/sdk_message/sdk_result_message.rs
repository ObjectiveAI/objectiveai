use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SDKResultMessage {
    Success(SDKResultSuccess),
    Error(SDKResultError),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SDKResultSuccess {
    #[serde(rename = "type")]
    pub r#type: String,
    pub subtype: String,
    pub duration_ms: i64,
    pub duration_api_ms: i64,
    pub is_error: bool,
    pub num_turns: i64,
    pub result: String,
    pub stop_reason: Option<String>,
    pub total_cost_usd: rust_decimal::Decimal,
    pub usage: super::super::beta_usage::NonNullableBetaUsage,
    #[serde(rename = "modelUsage")]
    pub model_usage: IndexMap<String, ModelUsage>,
    pub permission_denials: Vec<SDKPermissionDenial>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode_state: Option<super::FastModeState>,
    pub uuid: String,
    pub session_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SDKResultError {
    #[serde(rename = "type")]
    pub r#type: String,
    pub subtype: SDKResultErrorSubtype,
    pub duration_ms: i64,
    pub duration_api_ms: i64,
    pub is_error: bool,
    pub num_turns: i64,
    pub stop_reason: Option<String>,
    pub total_cost_usd: rust_decimal::Decimal,
    pub usage: super::super::beta_usage::NonNullableBetaUsage,
    #[serde(rename = "modelUsage")]
    pub model_usage: IndexMap<String, ModelUsage>,
    pub permission_denials: Vec<SDKPermissionDenial>,
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode_state: Option<super::FastModeState>,
    pub uuid: String,
    pub session_id: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SDKResultErrorSubtype {
    ErrorDuringExecution,
    ErrorMaxTurns,
    ErrorMaxBudgetUsd,
    ErrorMaxStructuredOutputRetries,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SDKPermissionDenial {
    pub tool_name: String,
    pub tool_use_id: String,
    pub tool_input: indexmap::IndexMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub web_search_requests: i64,
    #[serde(rename = "costUSD")]
    pub cost_usd: rust_decimal::Decimal,
    pub context_window: i64,
    pub max_output_tokens: i64,
}

impl SDKResultMessage {
    /// Returns the session ID from the result message.
    pub fn session_id(&self) -> &str {
        match self {
            SDKResultMessage::Success(s) => &s.session_id,
            SDKResultMessage::Error(e) => &e.session_id,
        }
    }

    /// Transforms this upstream result message into a downstream
    /// [`AgentCompletionChunk`] with final usage and cost information.
    #[allow(clippy::too_many_arguments)]
    pub fn into_downstream(
        self,
        id: String,
        created: u64,
        assistant_index: u64,
        is_byok: bool,
        cost_multiplier: rust_decimal::Decimal,
        upstream: objectiveai_sdk::agent::Upstream,
        agent_instance_hierarchy: String,
        agent_id: String,
        agent_full_id: String,
        agent_remote: Option<objectiveai_sdk::RemotePath>,
    ) -> objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        let upstream_id = self.session_id().to_string();
        let (total_cost_usd, usage, error) = match &self {
            SDKResultMessage::Success(s) => (s.total_cost_usd, &s.usage, None),
            SDKResultMessage::Error(e) => (
                e.total_cost_usd,
                &e.usage,
                Some(objectiveai_sdk::error::ResponseError {
                    code: 500,
                    message: serde_json::Value::String(
                        e.errors.join("; "),
                    ),
                }),
            ),
        };

        let prompt_tokens = (usage.input_tokens
            + usage.cache_creation_input_tokens
            + usage.cache_read_input_tokens) as u64;
        let completion_tokens = usage.output_tokens as u64;
        let total_tokens = prompt_tokens + completion_tokens;

        let prompt_tokens_details =
            Some(objectiveai_sdk::agent::completions::response::PromptTokensDetails {
                audio_tokens: None,
                cached_tokens: Some(usage.cache_read_input_tokens as u64),
                cache_write_tokens: Some(usage.cache_creation_input_tokens as u64),
                video_tokens: None,
            });

        // For Claude Agent SDK, Anthropic is the direct upstream with no intermediary,
        // so upstream_inference_cost = total_cost_usd and there is no upstream's upstream.
        let upstream_inference_cost = total_cost_usd;
        let upstream_upstream_inference_cost = rust_decimal::Decimal::ZERO;
        let upstream_total_cost = upstream_inference_cost + upstream_upstream_inference_cost;
        let total_cost = upstream_total_cost * cost_multiplier;
        let (cost, cost_details, total_cost) = if is_byok {
            (
                total_cost - upstream_total_cost,
                Some(objectiveai_sdk::agent::completions::response::CostDetails {
                    upstream_inference_cost,
                    upstream_upstream_inference_cost,
                }),
                total_cost,
            )
        } else {
            (total_cost, None, total_cost)
        };

        let downstream_usage = objectiveai_sdk::agent::completions::response::UpstreamUsage {
            completion_tokens,
            prompt_tokens,
            total_tokens,
            completion_tokens_details: None,
            prompt_tokens_details,
            cost,
            cost_details,
            total_cost,
            cost_multiplier,
            is_byok,
        };

        objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id,
            agent_instance_hierarchy,
            agent_id,
            agent_full_id,
            agent_remote,
            agent_inline: None,
            created,
            messages: vec![
                objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                    objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                        index: assistant_index,
                        created,
                        upstream_id,
                        usage: Some(downstream_usage),
                        ..Default::default()
                    },
                ),
            ],
            object: Default::default(),
            usage: None,
            upstream,
            error,
            continuation: None,
            messages_queued: None,
        }
    }
}
