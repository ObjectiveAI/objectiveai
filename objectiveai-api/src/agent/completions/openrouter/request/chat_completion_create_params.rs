//! Chat completion request parameters for OpenRouter.

use std::collections::HashMap;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// One entry in OpenRouter's request-body `plugins` array. Today the
/// only producer is `context-compression` (see the agent's
/// `context_compression` field), but the shape is OpenRouter-defined
/// — any future plugin id slots in here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
}

/// Chat completion request parameters formatted for the OpenRouter API.
///
/// Combines parameters from both the Agent configuration and the
/// incoming request to create a complete request for OpenRouter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionCreateParams {
    /// Messages for the conversation: the agent's system prompt (if any) as the
    /// leading entry, followed by the conversation (including any prefix/suffix
    /// from the Agent).
    pub messages: Vec<super::RequestMessage>,
    /// Provider preferences merged from request and Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<super::Provider>,

    /// The model identifier from the Agent.
    pub model: String,
    /// Frequency penalty from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Logit bias from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<IndexMap<String, i64>>,
    /// Maximum completion tokens from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    /// Presence penalty from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Stop sequences from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<objectiveai_sdk::agent::openrouter::Stop>,
    /// Temperature from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Top-p (nucleus sampling) from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Maximum tokens (legacy) from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Min-p sampling from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f64>,
    /// Reasoning configuration from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<objectiveai_sdk::agent::openrouter::Reasoning>,
    /// Repetition penalty from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    /// Top-a sampling from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_a: Option<f64>,
    /// Top-k sampling from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
    /// Verbosity setting from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<objectiveai_sdk::agent::openrouter::Verbosity>,
    /// Plugins array derived from Agent (currently only sourced from
    /// `context_compression`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<Plugin>>,

    /// Whether to include log probabilities from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// Number of top log probabilities to return from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u64>,
    /// Response format specification (never ToolCall — that variant is extracted into tools).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<super::response_format::ResponseFormat>,
    /// Random seed from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Tool choice configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<super::tool_choice::ToolChoice>,
    /// Available tools (MCP + response format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<super::Tool>>,
    /// Whether to allow parallel tool calls from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Prediction hints from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<super::Prediction>,

    /// Always true for streaming requests.
    pub stream: bool,
    /// Stream options for usage inclusion.
    pub stream_options: super::StreamOptions,
    /// Usage reporting options.
    pub usage: super::Usage,
}

impl ChatCompletionCreateParams {
    /// Creates request parameters from pre-resolved tool names and map.
    pub fn new(
        agent: &objectiveai_sdk::agent::openrouter::Agent,
        params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        continuation: Option<&[crate::agent::completions::ContinuationItem<objectiveai_sdk::agent::completions::message::AssistantMessage>]>,
        request_continuation: Option<&objectiveai_sdk::agent::openrouter::Continuation>,
        tool_names: &[String],
        tool_map: &HashMap<String, crate::agent::completions::resolved_tool::ResolvedTool>,
        tools_enabled: bool,
    ) -> Self {
        use crate::agent::completions::ContinuationItem;
        use objectiveai_sdk::agent::completions::message::Message;
        use super::RequestMessage;

        // --- Step 0: Build messages array (system_prompt + request_continuation + messages + continuation) ---
        // The agent's system prompt always leads, sourced live from the agent
        // on every turn (fresh or resumed). It is deliberately NOT part of the
        // continuation (`response_continuation` never stores it), so resuming
        // re-prepends the current agent's prompt exactly once with no
        // duplication.
        let continuation = continuation.unwrap_or_default();
        let sys_len = usize::from(agent.base.system_prompt.is_some());
        let rc_len = request_continuation.map_or(0, |rc| rc.messages.len());
        let mut all_messages = Vec::with_capacity(
            sys_len + rc_len + messages.len() + continuation.len(),
        );
        if let Some(system_prompt) = &agent.base.system_prompt {
            all_messages.push(RequestMessage::System(system_prompt.clone()));
        }
        if let Some(rc) = request_continuation {
            all_messages.extend(
                rc.messages
                    .iter()
                    .cloned()
                    .map(RequestMessage::Conversation),
            );
        }
        all_messages.extend(
            messages.iter().cloned().map(RequestMessage::Conversation),
        );
        all_messages.extend(continuation.iter().map(|item| {
            RequestMessage::Conversation(match item {
                ContinuationItem::State(assistant) => {
                    Message::Assistant(assistant.clone())
                }
                ContinuationItem::ToolMessage(tool) => {
                    Message::Tool(tool.clone())
                }
                ContinuationItem::UserMessage(user) => {
                    Message::User(user.clone())
                }
            })
        }));

        // --- Step 1: Resolve response_format for this agent ---
        let resolved_response_format = resolve_response_format(params, agent);

        // --- Step 2: Extract ToolCall variant (if any) from response_format ---
        let (openrouter_response_format, response_format_tool_required) =
            match resolved_response_format {
                Some(objectiveai_sdk::agent::completions::request::ResponseFormat::ToolCall {
                    name,
                    required,
                    ..
                }) => (None, Some((name, required))),
                Some(rf) => (Some(super::response_format::ResponseFormat::new(&rf)), None),
                None => (None, None),
            };

        // --- Step 3: Convert resolved tools to OpenRouter tools ---
        let final_tools: Vec<super::Tool> = tool_names
            .iter()
            .filter_map(|resolved_name| {
                let resolved = tool_map.get(resolved_name)?;
                Some(match resolved {
                    crate::agent::completions::resolved_tool::ResolvedTool::Mcp { tool, .. } => {
                        super::Tool::new_from_mcp(resolved_name.clone(), tool)
                    }
                    crate::agent::completions::resolved_tool::ResolvedTool::ResponseFormat {
                        description, schema,
                    } => {
                        super::Tool::new_from_response_format(
                            resolved_name.clone(),
                            description.clone(),
                            schema.clone(),
                        )
                    }
                })
            })
            .collect();

        // --- Step 4: Determine tool_choice ---
        let (tools, tool_choice) = if !tools_enabled {
            // When tools are disabled, send tool_choice: none.
            // Still include tool definitions so the model knows what
            // exists, but it is not allowed to call them.
            if final_tools.is_empty() {
                (None, None)
            } else {
                (Some(final_tools), Some(super::tool_choice::ToolChoice::None))
            }
        } else if final_tools.is_empty() {
            (None, None)
        } else if let Some((ref name, required)) = response_format_tool_required {
            let choice = if required == Some(true) {
                super::tool_choice::ToolChoice::Function(
                    super::tool_choice::ToolChoiceFunction::Function {
                        function:
                            super::tool_choice::ToolChoiceFunctionFunction {
                                name: name.clone(),
                            },
                    },
                )
            } else {
                super::tool_choice::ToolChoice::Auto
            };
            (Some(final_tools), Some(choice))
        } else {
            (
                Some(final_tools),
                Some(super::tool_choice::ToolChoice::Auto),
            )
        };

        Self {
            messages: all_messages,
            provider: super::provider::Provider::new(
                params.provider,
                agent.base.provider.as_ref(),
            ),
            model: agent.base.model.clone(),
            frequency_penalty: agent.base.frequency_penalty,
            logit_bias: agent.base.logit_bias.clone(),
            max_completion_tokens: agent.base.max_completion_tokens,
            presence_penalty: agent.base.presence_penalty,
            stop: agent.base.stop.clone(),
            temperature: agent.base.temperature,
            top_p: agent.base.top_p,
            max_tokens: agent.base.max_tokens,
            min_p: agent.base.min_p,
            reasoning: agent.base.reasoning,
            repetition_penalty: agent.base.repetition_penalty,
            top_a: agent.base.top_a,
            top_k: agent.base.top_k,
            verbosity: agent.base.verbosity,
            plugins: agent.base.context_compression.map(|cc| {
                let engine = serde_json::to_value(cc)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from));
                vec![Plugin {
                    id: "context-compression".to_string(),
                    engine,
                }]
            }),
            logprobs: if let Some(top_logprobs) = agent.base.top_logprobs && top_logprobs > 0 {
                Some(true)
            } else {
                None
            },
            top_logprobs: if let Some(top_logprobs) = agent.base.top_logprobs && top_logprobs > 0 {
                Some(top_logprobs)
            } else {
                None
            },
            response_format: openrouter_response_format,
            seed: params.seed,
            tool_choice,
            tools,
            parallel_tool_calls: None,
            prediction: None,
            stream: true,
            stream_options: super::StreamOptions {
                include_usage: Some(true),
            },
            usage: super::Usage { include: true },
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Resolves the response format for a specific agent from the request params.
fn resolve_response_format(
    params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
    agent: &objectiveai_sdk::agent::openrouter::Agent,
) -> Option<objectiveai_sdk::agent::completions::request::ResponseFormat> {
    match params.response_format.as_ref()? {
        objectiveai_sdk::agent::completions::request::ResponseFormatParam::Single(rf) => {
            Some(rf.clone())
        }
        objectiveai_sdk::agent::completions::request::ResponseFormatParam::PerAgent(map) => {
            map.get(&agent.id).cloned()
        }
    }
}

