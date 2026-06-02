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
    pub session_id: String,
}

impl SDKUserMessage {
    /// Transforms this upstream user message into a downstream
    /// [`AgentCompletionChunk`], or `None` if not a tool response.
    ///
    /// Only produces a chunk when both `tool_use_result` and
    /// `parent_tool_use_id` are present.
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
        let (Some(tool_use_result), Some(tool_call_id)) =
            (self.tool_use_result, self.parent_tool_use_id)
        else {
            return None;
        };

        let content_str = serde_json::to_string(&tool_use_result).unwrap_or_default();
        let message = objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Tool(
            objectiveai_sdk::agent::completions::response::ToolResponse {
                role: Default::default(),
                index: message_index,
                inner: objectiveai_sdk::agent::completions::message::ToolMessage {
                    content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                        content_str,
                    ),
                    tool_call_id,
                    metadata: None,
                },
            },
        );

        Some(
            objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                id,
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
                created,
                messages: vec![message],
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
