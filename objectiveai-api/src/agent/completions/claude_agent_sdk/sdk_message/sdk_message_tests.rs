use super::*;
use rust_decimal::Decimal;
use std::str::FromStr;

/// 1. Text delta produces an assistant chunk with content.
#[test]
fn test_text_delta() {
    let msg = SDKMessage::PartialAssistantMessage(SDKPartialAssistantMessage {
        r#type: SDKPartialAssistantMessageType::StreamEvent,
        event: super::super::beta_raw_message_stream_event::BetaRawMessageStreamEvent::ContentBlockDelta(
            super::super::beta_raw_message_stream_event::BetaRawContentBlockDeltaEvent {
                delta: super::super::beta_raw_message_stream_event::BetaRawContentBlockDelta::Text(
                    super::super::beta_raw_message_stream_event::BetaTextDelta {
                        text: "Hello world".to_string(),
                        r#type: super::super::beta_raw_message_stream_event::BetaTextDeltaType::TextDelta,
                    },
                ),
                index: 0,
                r#type: super::super::beta_raw_message_stream_event::BetaRawContentBlockDeltaEventType::ContentBlockDelta,
            },
        ),
        parent_tool_use_id: None,
        uuid: "uuid-1".to_string(),
        session_id: "sess-1".to_string(),
    });

    assert_eq!(
        msg.into_downstream("id-1".to_string(), 1000, 0, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        Some(Ok(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id: "id-1".to_string(),
            agent_instance_hierarchy: String::new(),
            agent_id: String::new(),
            agent_full_id: String::new(),
            agent_remote: None,
            created: 1000,
            messages: vec![
                objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                    objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                        role: Default::default(),
                        index: 0,
                        created: 1000,
                        upstream_id: "sess-1".to_string(),
                        content: Some(objectiveai_sdk::agent::completions::message::RichContent::Text("Hello world".to_string())),
                        ..Default::default()
                    },
                ),
            ],
            object: Default::default(),
            usage: None,
            upstream: objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
            error: None,
            continuation: None,
            messages_queued: None,
        }))
    );
}

/// 2. Thinking delta produces reasoning at the given assistant_index.
#[test]
fn test_thinking_delta() {
    let msg = SDKMessage::PartialAssistantMessage(SDKPartialAssistantMessage {
        r#type: SDKPartialAssistantMessageType::StreamEvent,
        event: super::super::beta_raw_message_stream_event::BetaRawMessageStreamEvent::ContentBlockDelta(
            super::super::beta_raw_message_stream_event::BetaRawContentBlockDeltaEvent {
                delta: super::super::beta_raw_message_stream_event::BetaRawContentBlockDelta::Thinking(
                    super::super::beta_raw_message_stream_event::BetaThinkingDelta {
                        thinking: "Let me consider...".to_string(),
                        r#type: super::super::beta_raw_message_stream_event::BetaThinkingDeltaType::ThinkingDelta,
                    },
                ),
                index: 0,
                r#type: super::super::beta_raw_message_stream_event::BetaRawContentBlockDeltaEventType::ContentBlockDelta,
            },
        ),
        parent_tool_use_id: None,
        uuid: "uuid-2".to_string(),
        session_id: "sess-2".to_string(),
    });

    assert_eq!(
        msg.into_downstream("id-2".to_string(), 2000, 3, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        Some(Ok(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id: "id-2".to_string(),
            agent_instance_hierarchy: String::new(),
            agent_id: String::new(),
            agent_full_id: String::new(),
            agent_remote: None,
            created: 2000,
            messages: vec![
                objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                    objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                        role: Default::default(),
                        index: 3,
                        created: 2000,
                        upstream_id: "sess-2".to_string(),
                        reasoning: Some("Let me consider...".to_string()),
                        ..Default::default()
                    },
                ),
            ],
            object: Default::default(),
            usage: None,
            upstream: objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
            error: None,
            continuation: None,
            messages_queued: None,
        }))
    );
}

/// 3. ToolUse content block start produces a tool call delta.
#[test]
fn test_tool_use_content_block_start() {
    let msg = SDKMessage::PartialAssistantMessage(SDKPartialAssistantMessage {
        r#type: SDKPartialAssistantMessageType::StreamEvent,
        event: super::super::beta_raw_message_stream_event::BetaRawMessageStreamEvent::ContentBlockStart(
            super::super::beta_raw_message_stream_event::BetaRawContentBlockStartEvent {
                content_block: super::super::beta_content_block::BetaContentBlock::ToolUse(
                    super::super::beta_content_block::BetaToolUseBlock {
                        id: "toolu_01".to_string(),
                        input: serde_json::Value::Object(Default::default()),
                        name: "read_file".to_string(),
                        r#type: super::super::beta_content_block::BetaToolUseBlockType::ToolUse,
                        caller: None,
                    },
                ),
                index: 2,
                r#type: super::super::beta_raw_message_stream_event::BetaRawContentBlockStartEventType::ContentBlockStart,
            },
        ),
        parent_tool_use_id: None,
        uuid: "uuid-3".to_string(),
        session_id: "sess-3".to_string(),
    });

    assert_eq!(
        msg.into_downstream("id-3".to_string(), 3000, 5, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        Some(Ok(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id: "id-3".to_string(),
            agent_instance_hierarchy: String::new(),
            agent_id: String::new(),
            agent_full_id: String::new(),
            agent_remote: None,
            created: 3000,
            messages: vec![
                objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                    objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                        role: Default::default(),
                        index: 5,
                        created: 3000,
                        upstream_id: "sess-3".to_string(),
                        tool_calls: Some(vec![
                            objectiveai_sdk::agent::completions::message::AssistantToolCallDelta {
                                index: 2,
                                r#type: Some(objectiveai_sdk::agent::completions::message::AssistantToolCallType::Function),
                                id: Some("toolu_01".to_string()),
                                function: Some(objectiveai_sdk::agent::completions::message::AssistantToolCallFunctionDelta {
                                    name: Some("read_file".to_string()),
                                    arguments: None,
                                }),
                            },
                        ]),
                        ..Default::default()
                    },
                ),
            ],
            object: Default::default(),
            usage: None,
            upstream: objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
            error: None,
            continuation: None,
            messages_queued: None,
        }))
    );
}

/// 4. InputJSON delta produces tool call arguments.
#[test]
fn test_input_json_delta() {
    let msg = SDKMessage::PartialAssistantMessage(SDKPartialAssistantMessage {
        r#type: SDKPartialAssistantMessageType::StreamEvent,
        event: super::super::beta_raw_message_stream_event::BetaRawMessageStreamEvent::ContentBlockDelta(
            super::super::beta_raw_message_stream_event::BetaRawContentBlockDeltaEvent {
                delta: super::super::beta_raw_message_stream_event::BetaRawContentBlockDelta::InputJSON(
                    super::super::beta_raw_message_stream_event::BetaInputJSONDelta {
                        partial_json: "{\"path\":\"src/".to_string(),
                        r#type: super::super::beta_raw_message_stream_event::BetaInputJSONDeltaType::InputJsonDelta,
                    },
                ),
                index: 2,
                r#type: super::super::beta_raw_message_stream_event::BetaRawContentBlockDeltaEventType::ContentBlockDelta,
            },
        ),
        parent_tool_use_id: None,
        uuid: "uuid-4".to_string(),
        session_id: "sess-4".to_string(),
    });

    assert_eq!(
        msg.into_downstream("id-4".to_string(), 4000, 0, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        Some(Ok(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id: "id-4".to_string(),
            agent_instance_hierarchy: String::new(),
            agent_id: String::new(),
            agent_full_id: String::new(),
            agent_remote: None,
            created: 4000,
            messages: vec![
                objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                    objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                        role: Default::default(),
                        index: 0,
                        created: 4000,
                        upstream_id: "sess-4".to_string(),
                        tool_calls: Some(vec![
                            objectiveai_sdk::agent::completions::message::AssistantToolCallDelta {
                                index: 2,
                                r#type: None,
                                id: None,
                                function: Some(objectiveai_sdk::agent::completions::message::AssistantToolCallFunctionDelta {
                                    name: None,
                                    arguments: Some("{\"path\":\"src/".to_string()),
                                }),
                            },
                        ]),
                        ..Default::default()
                    },
                ),
            ],
            object: Default::default(),
            usage: None,
            upstream: objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
            error: None,
            continuation: None,
            messages_queued: None,
        }))
    );
}

/// 5. MessageDelta with ToolUse stop reason produces ToolCalls finish reason.
#[test]
fn test_message_delta_tool_use_stop_reason() {
    let msg = SDKMessage::PartialAssistantMessage(SDKPartialAssistantMessage {
        r#type: SDKPartialAssistantMessageType::StreamEvent,
        event: super::super::beta_raw_message_stream_event::BetaRawMessageStreamEvent::MessageDelta(
            super::super::beta_raw_message_stream_event::BetaRawMessageDeltaEvent {
                context_management: None,
                delta: super::super::beta_raw_message_stream_event::BetaRawMessageDeltaEventDelta {
                    container: None,
                    stop_reason: Some(super::super::beta_message::BetaStopReason::ToolUse),
                    stop_sequence: None,
                },
                r#type: super::super::beta_raw_message_stream_event::BetaRawMessageDeltaEventType::MessageDelta,
                usage: super::super::beta_raw_message_stream_event::BetaMessageDeltaUsage {
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                    input_tokens: None,
                    iterations: None,
                    output_tokens: 150,
                    server_tool_use: None,
                },
            },
        ),
        parent_tool_use_id: None,
        uuid: "uuid-5".to_string(),
        session_id: "sess-5".to_string(),
    });

    assert_eq!(
        msg.into_downstream("id-5".to_string(), 5000, 0, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        Some(Ok(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id: "id-5".to_string(),
            agent_instance_hierarchy: String::new(),
            agent_id: String::new(),
            agent_full_id: String::new(),
            agent_remote: None,
            created: 5000,
            messages: vec![
                objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                    objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                        role: Default::default(),
                        index: 0,
                        created: 5000,
                        upstream_id: "sess-5".to_string(),
                        finish_reason: Some(objectiveai_sdk::agent::completions::response::FinishReason::ToolCalls),
                        ..Default::default()
                    },
                ),
            ],
            object: Default::default(),
            usage: None,
            upstream: objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
            error: None,
            continuation: None,
            messages_queued: None,
        }))
    );
}

/// 6. ContentBlockStop and MessageStop are ignored.
#[test]
fn test_content_block_stop_and_message_stop_ignored() {
    let stop = SDKMessage::PartialAssistantMessage(SDKPartialAssistantMessage {
        r#type: SDKPartialAssistantMessageType::StreamEvent,
        event: super::super::beta_raw_message_stream_event::BetaRawMessageStreamEvent::ContentBlockStop(
            super::super::beta_raw_message_stream_event::BetaRawContentBlockStopEvent {
                index: 0,
                r#type: super::super::beta_raw_message_stream_event::BetaRawContentBlockStopEventType::ContentBlockStop,
            },
        ),
        parent_tool_use_id: None,
        uuid: "uuid-6a".to_string(),
        session_id: "sess-6".to_string(),
    });

    assert_eq!(
        stop.into_downstream("id-6".to_string(), 6000, 0, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        None,
    );

    let msg_stop = SDKMessage::PartialAssistantMessage(SDKPartialAssistantMessage {
        r#type: SDKPartialAssistantMessageType::StreamEvent,
        event: super::super::beta_raw_message_stream_event::BetaRawMessageStreamEvent::MessageStop(
            super::super::beta_raw_message_stream_event::BetaRawMessageStopEvent {
                r#type: super::super::beta_raw_message_stream_event::BetaRawMessageStopEventType::MessageStop,
            },
        ),
        parent_tool_use_id: None,
        uuid: "uuid-6b".to_string(),
        session_id: "sess-6".to_string(),
    });

    assert_eq!(
        msg_stop.into_downstream("id-6".to_string(), 6000, 0, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        None,
    );
}

/// 7. UserMessage with tool_use_result produces a tool response at the given message_index.
#[test]
fn test_user_message_tool_result() {
    let msg = SDKMessage::UserMessage(SDKUserMessage {
        r#type: SDKUserMessageType::User,
        message: MessageParam {
            content: MessageParamContent::String("tool result".to_string()),
            role: MessageParamRole::User,
        },
        parent_tool_use_id: Some("toolu_abc".to_string()),
        is_synthetic: Some(true),
        tool_use_result: Some(serde_json::json!({"output": "file contents"})),
        uuid: Some("uuid-7".to_string()),
        session_id: "sess-7".to_string(),
    });

    assert_eq!(
        msg.into_downstream("id-7".to_string(), 7000, 4, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        Some(Ok(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id: "id-7".to_string(),
            agent_instance_hierarchy: String::new(),
            agent_id: String::new(),
            agent_full_id: String::new(),
            agent_remote: None,
            created: 7000,
            messages: vec![
                objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Tool(
                    objectiveai_sdk::agent::completions::response::ToolResponse {
                        role: Default::default(),
                        index: 4,
                        inner: objectiveai_sdk::agent::completions::message::ToolMessage {
                            content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                                "{\"output\":\"file contents\"}".to_string(),
                            ),
                            tool_call_id: "toolu_abc".to_string(),
                            metadata: None,
                        },
                    },
                ),
            ],
            object: Default::default(),
            usage: None,
            upstream: objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
            error: None,
            continuation: None,
            messages_queued: None,
        }))
    );
}

/// 8. UserMessage without tool_use_result is ignored.
#[test]
fn test_user_message_without_tool_result_ignored() {
    let msg = SDKMessage::UserMessage(SDKUserMessage {
        r#type: SDKUserMessageType::User,
        message: MessageParam {
            content: MessageParamContent::String("hello".to_string()),
            role: MessageParamRole::User,
        },
        parent_tool_use_id: None,
        is_synthetic: None,
        tool_use_result: None,
        uuid: Some("uuid-8".to_string()),
        session_id: "sess-8".to_string(),
    });

    assert_eq!(
        msg.into_downstream("id-8".to_string(), 8000, 0, false, Decimal::from(1), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        None,
    );
}

/// 9. RateLimitEvent is ignored by into_downstream. Rate-limit handling is
/// the caller's responsibility — the claude_agent_sdk runners handle them
/// entirely inside the subprocess.
#[test]
fn test_rate_limit_event_is_ignored() {
    let msg = SDKMessage::RateLimitEvent(SDKRateLimitEvent {
        r#type: RateLimitEventType::RateLimitEvent,
        rate_limit_info: None,
    });

    let result = msg.into_downstream(
        "id-9".to_string(), 9000, 0, false, Decimal::from(1),
        objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    );

    assert!(result.is_none());
}

/// 10. ResultMessage::Success with BYOK produces correct cost split and token details.
#[test]
fn test_result_success_byok() {
    let msg = SDKMessage::ResultMessage(SDKResultMessage::Success(SDKResultSuccess {
        r#type: "result".to_string(),
        subtype: "success".to_string(),
        duration_ms: 5000,
        duration_api_ms: 4500,
        is_error: false,
        num_turns: 3,
        result: "done".to_string(),
        stop_reason: Some("end_turn".to_string()),
        total_cost_usd: Decimal::from_str("0.05").unwrap(),
        usage: super::super::beta_usage::NonNullableBetaUsage {
            cache_creation: super::super::beta_usage::BetaCacheCreation {
                ephemeral_1h_input_tokens: 0,
                ephemeral_5m_input_tokens: 0,
            },
            cache_creation_input_tokens: 1000,
            cache_read_input_tokens: 5000,
            inference_geo: "us".to_string(),
            input_tokens: 2000,
            iterations: vec![],
            output_tokens: 800,
            server_tool_use: super::super::beta_usage::BetaServerToolUsage {
                web_fetch_requests: 0,
                web_search_requests: 0,
            },
            service_tier: super::super::beta_usage::ServiceTier::Standard,
            speed: super::super::beta_usage::Speed::Standard,
        },
        model_usage: indexmap::IndexMap::new(),
        permission_denials: vec![],
        structured_output: None,
        fast_mode_state: None,
        uuid: "uuid-10".to_string(),
        session_id: "sess-10".to_string(),
    }));

    assert_eq!(
        msg.into_downstream("id-10".to_string(), 10000, 0, true, Decimal::from_str("1.5").unwrap(), objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
        String::new(),
        String::new(),
        String::new(),
        None,
    ),
        Some(Ok(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
            id: "id-10".to_string(),
            agent_instance_hierarchy: String::new(),
            agent_id: String::new(),
            agent_full_id: String::new(),
            agent_remote: None,
            created: 10000,
            messages: vec![
                objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                    objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                        index: 0,
                        created: 10000,
                        upstream_id: "sess-10".to_string(),
                        usage: Some(objectiveai_sdk::agent::completions::response::UpstreamUsage {
                            // prompt = input(2000) + cache_creation(1000) + cache_read(5000) = 8000
                            prompt_tokens: 8000,
                            completion_tokens: 800,
                            total_tokens: 8800,
                            completion_tokens_details: None,
                            prompt_tokens_details: Some(objectiveai_sdk::agent::completions::response::PromptTokensDetails {
                                audio_tokens: None,
                                cached_tokens: Some(5000),
                                cache_write_tokens: Some(1000),
                                video_tokens: None,
                            }),
                            // upstream = 0.05, upstream_upstream = 0, total = 0.05 * 1.5 = 0.075
                            // byok cost = 0.075 - 0.05 = 0.025
                            cost: Decimal::from_str("0.025").unwrap(),
                            cost_details: Some(objectiveai_sdk::agent::completions::response::CostDetails {
                                upstream_inference_cost: Decimal::from_str("0.05").unwrap(),
                                upstream_upstream_inference_cost: Decimal::ZERO,
                            }),
                            total_cost: Decimal::from_str("0.075").unwrap(),
                            cost_multiplier: Decimal::from_str("1.5").unwrap(),
                            is_byok: true,
                        }),
                        ..Default::default()
                    },
                ),
            ],
            object: Default::default(),
            usage: None,
            upstream: objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
            error: None,
            continuation: None,
            messages_queued: None,
        }))
    );
}
