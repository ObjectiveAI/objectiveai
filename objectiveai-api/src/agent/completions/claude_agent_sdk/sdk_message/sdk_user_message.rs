use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SDKUserMessageType {
    User,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageParamRole {
    User,
    Assistant,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageParamContent {
    String(String),
    Blocks(Vec<super::super::content_block_param::ContentBlockParam>),
}

impl MessageParamContent {
    pub fn push(&mut self, block: super::super::content_block_param::ContentBlockParam) {
        match self {
            MessageParamContent::String(s) => {
                let mut blocks = if s.is_empty() {
                    vec![]
                } else {
                    vec![super::super::content_block_param::ContentBlockParam::Text(
                        super::super::content_block_param::TextBlockParam {
                            text: std::mem::take(s),
                            r#type: super::super::content_block_param::TextBlockParamType::Text,
                            cache_control: None,
                            citations: None,
                        },
                    )]
                };
                blocks.push(block);
                *self = MessageParamContent::Blocks(blocks);
            }
            MessageParamContent::Blocks(blocks) => {
                blocks.push(block);
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MessageParam {
    pub content: MessageParamContent,
    pub role: MessageParamRole,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SDKUserMessage {
    pub r#type: SDKUserMessageType,
    pub message: MessageParam,
    pub parent_tool_use_id: Option<String>,
    #[serde(rename = "isSynthetic", skip_serializing_if = "Option::is_none")]
    pub is_synthetic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    // The SDK's `UserMessage` dataclass carries no `session_id`, so the Python
    // runner never emits one for tool-result user messages. Optional, or those
    // lines fail to deserialize and are silently dropped before logging.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl SDKUserMessage {
    /// Transforms this upstream user message into a downstream
    /// [`AgentCompletionChunk`] carrying one tool response per `tool_result`,
    /// or `None` when the message is not a tool result.
    ///
    /// The tool-call id and content come from the `tool_result` block(s) in
    /// `message.content` — the shape the runner actually emits for a direct
    /// tool call (which carries NO `parent_tool_use_id`). As a fallback, a
    /// structured top-level `tool_use_result` keyed by `parent_tool_use_id`
    /// (sub-agent / Task results that arrive without an inline block) is still
    /// honored.
    #[allow(clippy::too_many_arguments)]
    pub fn into_downstream(
        self,
        id: String,
        created: u64,
        message_index: u64,
        upstream: objectiveai_sdk::agent::Upstream,
        agent_instance_hierarchy: String,
        agent_id: String,
        agent_full_id: String,
        agent_remote: Option<objectiveai_sdk::RemotePath>,
    ) -> Option<objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk> {
        let mut messages = Vec::new();

        // Primary: inline `tool_result` blocks (direct tool calls). The
        // tool-call id is the block's own `tool_use_id`.
        if let MessageParamContent::Blocks(blocks) = &self.message.content {
            for block in blocks {
                if let super::super::content_block_param::ContentBlockParam::ToolResult(tr) =
                    block
                {
                    messages.push(tool_response_chunk(
                        message_index,
                        tr.tool_use_id.clone(),
                        render_tool_result_content(tr.content.as_ref()),
                    ));
                }
            }
        }

        // Fallback: a structured `tool_use_result` keyed by
        // `parent_tool_use_id` (sub-agent / Task results with no inline
        // `tool_result` block).
        if messages.is_empty() {
            if let (Some(tool_use_result), Some(tool_call_id)) =
                (&self.tool_use_result, &self.parent_tool_use_id)
            {
                messages.push(tool_response_chunk(
                    message_index,
                    tool_call_id.clone(),
                    serde_json::to_string(tool_use_result).unwrap_or_default(),
                ));
            }
        }

        if messages.is_empty() {
            return None;
        }

        Some(
            objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                id,
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
                agent_inline: None,
                created,
                messages,
                object: Default::default(),
                usage: None,
                upstream,
                error: None,
                continuation: None,
                messages_queued: None,
            },
        )
    }
}

/// Builds one tool-response message chunk.
fn tool_response_chunk(
    index: u64,
    tool_call_id: String,
    content: String,
) -> objectiveai_sdk::agent::completions::response::streaming::MessageChunk {
    objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Tool(
        objectiveai_sdk::agent::completions::response::ToolResponse {
            role: Default::default(),
            index,
            inner: objectiveai_sdk::agent::completions::message::ToolMessage {
                content: objectiveai_sdk::agent::completions::message::RichContent::Text(content),
                tool_call_id,
                metadata: None,
            },
            request_message_ids: None,
        },
    )
}

/// Renders a `tool_result` block's content to plain text — text blocks
/// concatenated in order; non-text blocks are skipped. `None` → empty string.
fn render_tool_result_content(
    content: Option<&super::super::content_block_param::ToolResultBlockParamContent>,
) -> String {
    use super::super::content_block_param::{
        ToolResultBlockParamContent, ToolResultContentBlockParam,
    };
    match content {
        None => String::new(),
        Some(ToolResultBlockParamContent::String(s)) => s.clone(),
        Some(ToolResultBlockParamContent::Blocks(blocks)) => blocks
            .iter()
            .filter_map(|b| match b {
                ToolResultContentBlockParam::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<String>(),
    }
}
