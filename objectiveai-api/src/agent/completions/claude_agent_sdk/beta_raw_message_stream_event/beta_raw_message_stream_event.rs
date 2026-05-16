use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BetaRawMessageStreamEvent {
    MessageStart(super::BetaRawMessageStartEvent),
    MessageDelta(super::BetaRawMessageDeltaEvent),
    MessageStop(super::BetaRawMessageStopEvent),
    ContentBlockStart(super::BetaRawContentBlockStartEvent),
    ContentBlockDelta(super::BetaRawContentBlockDeltaEvent),
    ContentBlockStop(super::BetaRawContentBlockStopEvent),
}

impl BetaRawMessageStreamEvent {
    /// Transforms this upstream stream event into a downstream
    /// [`AgentCompletionChunk`], or `None` if the event should be ignored.
    pub fn into_downstream(
        self,
        id: String,
        created: u64,
        agent: String,
        assistant_index: u64,
        session_id: String,
        upstream: objectiveai_sdk::agent::Upstream,
    ) -> Option<objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk> {
        use objectiveai_sdk::agent::completions::message;
        use objectiveai_sdk::agent::completions::response;

        let message_chunk = match self {
            // MessageStart: extract model, use session_id as upstream_id
            Self::MessageStart(event) => {
                let msg = event.message;
                Some(response::streaming::MessageChunk::Assistant(
                    response::streaming::AssistantResponseChunk {
                        role: Default::default(),
                        index: assistant_index,
                        created,
                        agent: agent.clone(),
                        model: msg.model,
                        upstream_id: session_id.clone(),
                        ..Default::default()
                    },
                ))
            }

            // ContentBlockStart: only ToolUse variants produce a chunk
            Self::ContentBlockStart(event) => {
                match event.content_block {
                    super::super::beta_content_block::BetaContentBlock::ToolUse(tool) => {
                        Some(response::streaming::MessageChunk::Assistant(
                            response::streaming::AssistantResponseChunk {
                                role: Default::default(),
                                index: assistant_index,
                                created,
                                agent: agent.clone(),
                                upstream_id: session_id.clone(),
                                tool_calls: Some(vec![message::AssistantToolCallDelta {
                                    index: event.index as u64,
                                    r#type: Some(message::AssistantToolCallType::Function),
                                    id: Some(tool.id),
                                    function: Some(message::AssistantToolCallFunctionDelta {
                                        name: Some(tool.name),
                                        arguments: None,
                                    }),
                                }]),
                                ..Default::default()
                            },
                        ))
                    }
                    super::super::beta_content_block::BetaContentBlock::MCPToolUse(tool) => {
                        Some(response::streaming::MessageChunk::Assistant(
                            response::streaming::AssistantResponseChunk {
                                role: Default::default(),
                                index: assistant_index,
                                created,
                                agent: agent.clone(),
                                upstream_id: session_id.clone(),
                                tool_calls: Some(vec![message::AssistantToolCallDelta {
                                    index: event.index as u64,
                                    r#type: Some(message::AssistantToolCallType::Function),
                                    id: Some(tool.id),
                                    function: Some(message::AssistantToolCallFunctionDelta {
                                        name: Some(tool.name),
                                        arguments: None,
                                    }),
                                }]),
                                ..Default::default()
                            },
                        ))
                    }
                    super::super::beta_content_block::BetaContentBlock::ServerToolUse(tool) => {
                        Some(response::streaming::MessageChunk::Assistant(
                            response::streaming::AssistantResponseChunk {
                                role: Default::default(),
                                index: assistant_index,
                                created,
                                agent: agent.clone(),
                                upstream_id: session_id.clone(),
                                tool_calls: Some(vec![message::AssistantToolCallDelta {
                                    index: event.index as u64,
                                    r#type: Some(message::AssistantToolCallType::Function),
                                    id: Some(tool.id),
                                    function: Some(message::AssistantToolCallFunctionDelta {
                                        name: Some(tool.name.as_str().into()),
                                        arguments: None,
                                    }),
                                }]),
                                ..Default::default()
                            },
                        ))
                    }
                    _ => None,
                }
            }

            // ContentBlockDelta: text, thinking, and input_json produce chunks
            Self::ContentBlockDelta(event) => {
                match event.delta {
                    super::BetaRawContentBlockDelta::Text(delta) => {
                        Some(response::streaming::MessageChunk::Assistant(
                            response::streaming::AssistantResponseChunk {
                                role: Default::default(),
                                index: assistant_index,
                                created,
                                agent: agent.clone(),
                                upstream_id: session_id.clone(),
                                content: Some(message::RichContent::Text(delta.text)),
                                ..Default::default()
                            },
                        ))
                    }
                    super::BetaRawContentBlockDelta::Thinking(delta) => {
                        Some(response::streaming::MessageChunk::Assistant(
                            response::streaming::AssistantResponseChunk {
                                role: Default::default(),
                                index: assistant_index,
                                created,
                                agent: agent.clone(),
                                upstream_id: session_id.clone(),
                                reasoning: Some(delta.thinking),
                                ..Default::default()
                            },
                        ))
                    }
                    super::BetaRawContentBlockDelta::InputJSON(delta) => {
                        Some(response::streaming::MessageChunk::Assistant(
                            response::streaming::AssistantResponseChunk {
                                role: Default::default(),
                                index: assistant_index,
                                created,
                                agent: agent.clone(),
                                upstream_id: session_id.clone(),
                                tool_calls: Some(vec![message::AssistantToolCallDelta {
                                    index: event.index as u64,
                                    r#type: None,
                                    id: None,
                                    function: Some(message::AssistantToolCallFunctionDelta {
                                        name: None,
                                        arguments: Some(delta.partial_json),
                                    }),
                                }]),
                                ..Default::default()
                            },
                        ))
                    }
                    _ => None,
                }
            }

            // MessageDelta: finish reason only (usage comes from ResultMessage)
            Self::MessageDelta(event) => {
                let finish_reason = event.delta.stop_reason.map(|sr| {
                    use super::super::beta_message::BetaStopReason;
                    match sr {
                        BetaStopReason::EndTurn => response::FinishReason::Stop,
                        BetaStopReason::MaxTokens => response::FinishReason::Length,
                        BetaStopReason::StopSequence => response::FinishReason::Stop,
                        BetaStopReason::ToolUse => response::FinishReason::ToolCalls,
                        BetaStopReason::PauseTurn => response::FinishReason::Stop,
                        BetaStopReason::Refusal => response::FinishReason::ContentFilter,
                        BetaStopReason::Compaction => response::FinishReason::Error,
                        BetaStopReason::ModelContextWindowExceeded => {
                            response::FinishReason::Error
                        }
                    }
                });
                Some(response::streaming::MessageChunk::Assistant(
                    response::streaming::AssistantResponseChunk {
                        role: Default::default(),
                        index: assistant_index,
                        created,
                        agent: agent.clone(),
                        upstream_id: session_id.clone(),
                        finish_reason,
                        ..Default::default()
                    },
                ))
            }

            // ContentBlockStop and MessageStop produce nothing
            Self::ContentBlockStop(_) | Self::MessageStop(_) => None,
        };

        message_chunk.map(|message| {
            objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                id,
                created,
                messages: vec![message],
                object: Default::default(),
                usage: None,
                upstream,
                error: None,
                continuation: None,
            }
        })
    }
}
