#[allow(unused_imports)]
use super::*;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Helper: builds a minimal ChatCompletionChunk with one choice.
fn chunk_with_delta(id: &str, model: &str, delta: Delta) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: id.to_string(),
        choices: vec![Choice {
            delta,
            finish_reason: None,
            index: 0,
            logprobs: None,
        }],
        created: 1000,
        model: model.to_string(),
        object: Object::default(),
        service_tier: None,
        system_fingerprint: None,
        usage: None,
        provider: None,
    }
}

#[test]
fn test_text_only_content() {
    let chunk = chunk_with_delta(
        "or-123",
        "openai/gpt-4o",
        Delta {
            content: Some("Hello".to_string()),
            ..Default::default()
        },
    );

    let result = chunk.into_downstream(
        "obj-1".to_string(),
        1000,
        "agent-a".to_string(),
        0,
        false,
        Decimal::from(1),
    );

    let expected = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        id: "obj-1".to_string(),
        created: 1000,
        messages: vec![
            objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                    role: Default::default(),
                    index: 0,
                    created: 1000,
                    agent: "agent-a".to_string(),
                    model: "openai/gpt-4o".to_string(),
                    upstream_id: "or-123".to_string(),
                    reasoning: None,
                    tool_calls: None,
                    content: Some(
                        objectiveai_sdk::agent::completions::message::RichContent::Text(
                            "Hello".to_string(),
                        ),
                    ),
                    refusal: None,
                    finish_reason: None,
                    logprobs: None,
                    service_tier: None,
                    system_fingerprint: None,
                    provider: None,
                    usage: None,
                },
            ),
        ],
        object: Default::default(),
        usage: None,
        upstream: objectiveai_sdk::agent::Upstream::Openrouter,
        error: None,
        continuation: None,
        messages_queued: None,
    };

    assert_eq!(result, expected);
}

#[test]
fn test_empty_delta() {
    let chunk = chunk_with_delta(
        "or-empty",
        "google/gemini-2.5-pro",
        Delta::default(),
    );

    let result = chunk.into_downstream(
        "obj-2".to_string(),
        1000,
        "agent-b".to_string(),
        0,
        false,
        Decimal::from(1),
    );

    let expected = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        id: "obj-2".to_string(),
        created: 1000,
        messages: vec![
            objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                    role: Default::default(),
                    index: 0,
                    created: 1000,
                    agent: "agent-b".to_string(),
                    model: "google/gemini-2.5-pro".to_string(),
                    upstream_id: "or-empty".to_string(),
                    reasoning: None,
                    tool_calls: None,
                    content: None,
                    refusal: None,
                    finish_reason: None,
                    logprobs: None,
                    service_tier: None,
                    system_fingerprint: None,
                    provider: None,
                    usage: None,
                },
            ),
        ],
        object: Default::default(),
        usage: None,
        upstream: objectiveai_sdk::agent::Upstream::Openrouter,
        error: None,
        continuation: None,
        messages_queued: None,
    };

    assert_eq!(result, expected);
}

#[test]
fn test_images_only() {
    let chunk = chunk_with_delta(
        "or-img",
        "openai/gpt-4o",
        Delta {
            images: Some(vec![
                Image::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/a.png".to_string(),
                    },
                },
                Image::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/b.png".to_string(),
                    },
                },
            ]),
            ..Default::default()
        },
    );

    let result = chunk.into_downstream(
        "obj-3".to_string(),
        1000,
        "agent-c".to_string(),
        0,
        false,
        Decimal::from(1),
    );

    let expected = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        id: "obj-3".to_string(),
        created: 1000,
        messages: vec![
            objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                    role: Default::default(),
                    index: 0,
                    created: 1000,
                    agent: "agent-c".to_string(),
                    model: "openai/gpt-4o".to_string(),
                    upstream_id: "or-img".to_string(),
                    reasoning: None,
                    tool_calls: None,
                    content: Some(
                        objectiveai_sdk::agent::completions::message::RichContent::Parts(vec![
                            objectiveai_sdk::agent::completions::message::RichContentPart::ImageUrl {
                                image_url: objectiveai_sdk::agent::completions::message::ImageUrl {
                                    url: "https://example.com/a.png".to_string(),
                                    detail: None,
                                },
                            },
                            objectiveai_sdk::agent::completions::message::RichContentPart::ImageUrl {
                                image_url: objectiveai_sdk::agent::completions::message::ImageUrl {
                                    url: "https://example.com/b.png".to_string(),
                                    detail: None,
                                },
                            },
                        ]),
                    ),
                    refusal: None,
                    finish_reason: None,
                    logprobs: None,
                    service_tier: None,
                    system_fingerprint: None,
                    provider: None,
                    usage: None,
                },
            ),
        ],
        object: Default::default(),
        usage: None,
        upstream: objectiveai_sdk::agent::Upstream::Openrouter,
        error: None,
        continuation: None,
        messages_queued: None,
    };

    assert_eq!(result, expected);
}

#[test]
fn test_text_and_images_merged() {
    let chunk = chunk_with_delta(
        "or-mix",
        "openai/gpt-4o",
        Delta {
            content: Some("Here is the image:".to_string()),
            images: Some(vec![Image::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/gen.png".to_string(),
                },
            }]),
            ..Default::default()
        },
    );

    let result = chunk.into_downstream(
        "obj-4".to_string(),
        1000,
        "agent-d".to_string(),
        0,
        false,
        Decimal::from(1),
    );

    let expected = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        id: "obj-4".to_string(),
        created: 1000,
        messages: vec![
            objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                    role: Default::default(),
                    index: 0,
                    created: 1000,
                    agent: "agent-d".to_string(),
                    model: "openai/gpt-4o".to_string(),
                    upstream_id: "or-mix".to_string(),
                    reasoning: None,
                    tool_calls: None,
                    content: Some(
                        objectiveai_sdk::agent::completions::message::RichContent::Parts(vec![
                            objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                                text: "Here is the image:".to_string(),
                            },
                            objectiveai_sdk::agent::completions::message::RichContentPart::ImageUrl {
                                image_url: objectiveai_sdk::agent::completions::message::ImageUrl {
                                    url: "https://example.com/gen.png".to_string(),
                                    detail: None,
                                },
                            },
                        ]),
                    ),
                    refusal: None,
                    finish_reason: None,
                    logprobs: None,
                    service_tier: None,
                    system_fingerprint: None,
                    provider: None,
                    usage: None,
                },
            ),
        ],
        object: Default::default(),
        usage: None,
        upstream: objectiveai_sdk::agent::Upstream::Openrouter,
        error: None,
        continuation: None,
        messages_queued: None,
    };

    assert_eq!(result, expected);
}

#[test]
fn test_usage_with_cost_multiplier() {
    let chunk = ChatCompletionChunk {
        id: "or-usage".to_string(),
        choices: vec![Choice {
            delta: Delta {
                content: Some("done".to_string()),
                ..Default::default()
            },
            finish_reason: Some(
                objectiveai_sdk::agent::completions::response::FinishReason::Stop,
            ),
            index: 0,
            logprobs: None,
        }],
        created: 2000,
        model: "openai/gpt-4o".to_string(),
        object: Object::default(),
        service_tier: Some("default".to_string()),
        system_fingerprint: Some("fp_abc123".to_string()),
        usage: Some(Usage {
            completion_tokens: 50,
            prompt_tokens: 100,
            total_tokens: 150,
            completion_tokens_details: None,
            prompt_tokens_details: None,
            cost: Some(Decimal::from_str("0.001").unwrap()),
            cost_details: None,
        }),
        provider: Some("OpenAI".to_string()),
    };

    let multiplier = Decimal::from_str("1.5").unwrap();
    let result = chunk.into_downstream(
        "obj-5".to_string(),
        2000,
        "agent-e".to_string(),
        0,
        false,
        multiplier,
    );

    let expected = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        id: "obj-5".to_string(),
        created: 2000,
        messages: vec![
            objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                    role: Default::default(),
                    index: 0,
                    created: 2000,
                    agent: "agent-e".to_string(),
                    model: "openai/gpt-4o".to_string(),
                    upstream_id: "or-usage".to_string(),
                    reasoning: None,
                    tool_calls: None,
                    content: Some(
                        objectiveai_sdk::agent::completions::message::RichContent::Text(
                            "done".to_string(),
                        ),
                    ),
                    refusal: None,
                    finish_reason: Some(
                        objectiveai_sdk::agent::completions::response::FinishReason::Stop,
                    ),
                    logprobs: None,
                    service_tier: Some("default".to_string()),
                    system_fingerprint: Some("fp_abc123".to_string()),
                    provider: Some("OpenAI".to_string()),
                    usage: Some(objectiveai_sdk::agent::completions::response::UpstreamUsage {
                        completion_tokens: 50,
                        prompt_tokens: 100,
                        total_tokens: 150,
                        completion_tokens_details: None,
                        prompt_tokens_details: None,
                        cost: Decimal::from_str("0.0015").unwrap(),
                        cost_details: None,
                        total_cost: Decimal::from_str("0.0015").unwrap(),
                        cost_multiplier: multiplier,
                        is_byok: false,
                    }),
                },
            ),
        ],
        object: Default::default(),
        usage: None,
        upstream: objectiveai_sdk::agent::Upstream::Openrouter,
        error: None,
        continuation: None,
        messages_queued: None,
    };

    assert_eq!(result, expected);
}

#[test]
fn test_reasoning_and_tool_calls() {
    let chunk = chunk_with_delta(
        "or-tools",
        "anthropic/claude-sonnet-4",
        Delta {
            reasoning: Some("Let me think...".to_string()),
            tool_calls: Some(vec![
                objectiveai_sdk::agent::completions::message::AssistantToolCallDelta {
                    index: 0,
                    id: Some("call_1".to_string()),
                    r#type: Some(
                        objectiveai_sdk::agent::completions::message::AssistantToolCallType::Function,
                    ),
                    function: Some(
                        objectiveai_sdk::agent::completions::message::AssistantToolCallFunctionDelta {
                            name: Some("get_weather".to_string()),
                            arguments: Some("{\"city\":\"NYC\"}".to_string()),
                        },
                    ),
                },
            ]),
            ..Default::default()
        },
    );

    let result = chunk.into_downstream(
        "obj-6".to_string(),
        1000,
        "agent-f".to_string(),
        0,
        false,
        Decimal::from(1),
    );

    let expected = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        id: "obj-6".to_string(),
        created: 1000,
        messages: vec![
            objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                    role: Default::default(),
                    index: 0,
                    created: 1000,
                    agent: "agent-f".to_string(),
                    model: "anthropic/claude-sonnet-4".to_string(),
                    upstream_id: "or-tools".to_string(),
                    reasoning: Some("Let me think...".to_string()),
                    tool_calls: Some(vec![
                        objectiveai_sdk::agent::completions::message::AssistantToolCallDelta {
                            index: 0,
                            id: Some("call_1".to_string()),
                            r#type: Some(
                                objectiveai_sdk::agent::completions::message::AssistantToolCallType::Function,
                            ),
                            function: Some(
                                objectiveai_sdk::agent::completions::message::AssistantToolCallFunctionDelta {
                                    name: Some("get_weather".to_string()),
                                    arguments: Some("{\"city\":\"NYC\"}".to_string()),
                                },
                            ),
                        },
                    ]),
                    content: None,
                    refusal: None,
                    finish_reason: None,
                    logprobs: None,
                    service_tier: None,
                    system_fingerprint: None,
                    provider: None,
                    usage: None,
                },
            ),
        ],
        object: Default::default(),
        usage: None,
        upstream: objectiveai_sdk::agent::Upstream::Openrouter,
        error: None,
        continuation: None,
        messages_queued: None,
    };

    assert_eq!(result, expected);
}

#[test]
fn test_byok_cost_splitting() {
    let chunk = ChatCompletionChunk {
        id: "or-byok".to_string(),
        choices: vec![Choice {
            delta: Delta::default(),
            finish_reason: Some(
                objectiveai_sdk::agent::completions::response::FinishReason::Stop,
            ),
            index: 0,
            logprobs: None,
        }],
        created: 3000,
        model: "openai/gpt-4o".to_string(),
        object: Object::default(),
        service_tier: None,
        system_fingerprint: None,
        usage: Some(Usage {
            completion_tokens: 10,
            prompt_tokens: 20,
            total_tokens: 30,
            completion_tokens_details: None,
            prompt_tokens_details: None,
            cost: Some(Decimal::from_str("0.01").unwrap()),
            cost_details: Some(CostDetails {
                upstream_inference_cost: Some(Decimal::from_str("0.008").unwrap()),
            }),
        }),
        provider: None,
    };

    let multiplier = Decimal::from_str("2.0").unwrap();
    let result = chunk.into_downstream(
        "obj-7".to_string(),
        3000,
        "agent-g".to_string(),
        0,
        true,
        multiplier,
    );

    // upstream_total = 0.01 + 0.008 = 0.018
    // total_cost = 0.018 * 2.0 = 0.036
    // cost (byok) = 0.036 - 0.018 = 0.018
    let expected = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
        id: "obj-7".to_string(),
        created: 3000,
        messages: vec![
            objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(
                objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunk {
                    role: Default::default(),
                    index: 0,
                    created: 3000,
                    agent: "agent-g".to_string(),
                    model: "openai/gpt-4o".to_string(),
                    upstream_id: "or-byok".to_string(),
                    reasoning: None,
                    tool_calls: None,
                    content: None,
                    refusal: None,
                    finish_reason: Some(
                        objectiveai_sdk::agent::completions::response::FinishReason::Stop,
                    ),
                    logprobs: None,
                    service_tier: None,
                    system_fingerprint: None,
                    provider: None,
                    usage: Some(objectiveai_sdk::agent::completions::response::UpstreamUsage {
                        completion_tokens: 10,
                        prompt_tokens: 20,
                        total_tokens: 30,
                        completion_tokens_details: None,
                        prompt_tokens_details: None,
                        cost: Decimal::from_str("0.018").unwrap(),
                        cost_details: Some(objectiveai_sdk::agent::completions::response::CostDetails {
                            upstream_inference_cost: Decimal::from_str("0.01").unwrap(),
                            upstream_upstream_inference_cost: Decimal::from_str("0.008").unwrap(),
                        }),
                        total_cost: Decimal::from_str("0.036").unwrap(),
                        cost_multiplier: multiplier,
                        is_byok: true,
                    }),
                },
            ),
        ],
        object: Default::default(),
        usage: None,
        upstream: objectiveai_sdk::agent::Upstream::Openrouter,
        error: None,
        continuation: None,
        messages_queued: None,
    };

    assert_eq!(result, expected);
}
