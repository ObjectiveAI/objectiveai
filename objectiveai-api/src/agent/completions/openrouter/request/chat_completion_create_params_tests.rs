//! Tests for [`ChatCompletionCreateParams`] construction.

use super::*;
use std::sync::Arc;

/// Helper to resolve tools and build params via the per-agent proxy
/// connection. Pass `None` when the test needs no tools.
async fn build_params(
    agent: &objectiveai_sdk::agent::openrouter::Agent,
    params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
    messages: &[objectiveai_sdk::agent::completions::message::Message],
    continuation: Option<&[crate::agent::completions::ContinuationItem<objectiveai_sdk::agent::completions::message::AssistantMessage>]>,
    mcp_connection: Option<&objectiveai_sdk::mcp::Connection>,
) -> ChatCompletionCreateParams {
    let resolved_rf = params.response_format.as_ref().and_then(|rfp| {
        match rfp {
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::Single(rf) => Some(rf.clone()),
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::PerAgent(map) => map.get(&agent.id).cloned(),
        }
    });
    let (tool_names, tool_map) = crate::agent::completions::resolved_tool::resolve_tools(
        mcp_connection,
        resolved_rf.as_ref(),
    )
    .await
    .expect("resolve_tools");
    ChatCompletionCreateParams::new(
        agent, params, messages, continuation, None,
        &tool_names, &tool_map, true,
    )
}

/// Like `build_params` but with explicit `tools_enabled` control.
async fn build_params_with_tools_enabled(
    agent: &objectiveai_sdk::agent::openrouter::Agent,
    params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
    messages: &[objectiveai_sdk::agent::completions::message::Message],
    continuation: Option<&[crate::agent::completions::ContinuationItem<objectiveai_sdk::agent::completions::message::AssistantMessage>]>,
    mcp_connection: Option<&objectiveai_sdk::mcp::Connection>,
    tools_enabled: bool,
) -> ChatCompletionCreateParams {
    let resolved_rf = params.response_format.as_ref().and_then(|rfp| {
        match rfp {
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::Single(rf) => Some(rf.clone()),
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::PerAgent(map) => map.get(&agent.id).cloned(),
        }
    });
    let (tool_names, tool_map) = crate::agent::completions::resolved_tool::resolve_tools(
        mcp_connection,
        resolved_rf.as_ref(),
    )
    .await
    .expect("resolve_tools");
    ChatCompletionCreateParams::new(
        agent, params, messages, continuation, None,
        &tool_names, &tool_map, tools_enabled,
    )
}

#[tokio::test]
async fn test_no_tools_empty_params() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai_sdk::agent::completions::message::Message::User(
            objectiveai_sdk::agent::completions::message::UserMessage {
                content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                    "Hello".into(),
                ),
                name: None,
            },
        ),
    ];

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        None,
    ).await;

    let expected = ChatCompletionCreateParams {
        messages: messages.clone(),
        provider: None,
        model: "test-model".into(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: None,
        tools: None,
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: super::StreamOptions {
            include_usage: Some(true),
        },
        usage: super::Usage { include: true },
    };

    assert_eq!(result, expected);
}


#[tokio::test]
async fn test_top_logprobs_zero_omits_logprobs() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent {
        id: String::new(),
        base: objectiveai_sdk::agent::openrouter::AgentBase {
            model: "test-model".to_string(),
            top_logprobs: Some(0),
            ..Default::default()
        },
    };

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages: Vec<objectiveai_sdk::agent::completions::message::Message> = vec![];
    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        None,
    ).await;

    assert_eq!(
        result,
        ChatCompletionCreateParams {
            messages: vec![],
            provider: None,
            model: "test-model".to_string(),
            frequency_penalty: None,
            logit_bias: None,
            max_completion_tokens: None,
            presence_penalty: None,
            stop: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            min_p: None,
            reasoning: None,
            repetition_penalty: None,
            top_a: None,
            top_k: None,
            verbosity: None,
            logprobs: None,
            top_logprobs: None,
            response_format: None,
            seed: None,
            tool_choice: None,
            tools: None,
            parallel_tool_calls: None,
            prediction: None,
            stream: true,
            stream_options: StreamOptions {
                include_usage: Some(true),
            },
            usage: Usage { include: true },
        }
    );
}

#[tokio::test]
async fn test_multiple_invention_tools_no_conflicts() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages = vec![objectiveai_sdk::agent::completions::message::Message::User(
        objectiveai_sdk::agent::completions::message::UserMessage {
            content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                "Hello".into(),
            ),
            name: None,
        },
    )];

    let search_params = {
        let mut m = indexmap::IndexMap::new();
        m.insert("type".into(), serde_json::json!("object"));
        m.insert(
            "properties".into(),
            serde_json::json!({"query": {"type": "string"}}),
        );
        m
    };
    let calculate_params = {
        let mut m = indexmap::IndexMap::new();
        m.insert("type".into(), serde_json::json!("object"));
        m.insert(
            "properties".into(),
            serde_json::json!({"expression": {"type": "string"}}),
        );
        m
    };
    let translate_params = {
        let mut m = indexmap::IndexMap::new();
        m.insert("type".into(), serde_json::json!("object"));
        m.insert(
            "properties".into(),
            serde_json::json!({"text": {"type": "string"}, "target_language": {"type": "string"}}),
        );
        m
    };

    let invention_tools = vec![
        objectiveai_sdk::functions::inventions::InventionTool {
            name: "search".to_string(),
            description: "Search the web",
            parameters: search_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("".into()) })),
        },
        objectiveai_sdk::functions::inventions::InventionTool {
            name: "calculate".to_string(),
            description: "Evaluate a math expression",
            parameters: calculate_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("".into()) })),
        },
        objectiveai_sdk::functions::inventions::InventionTool {
            name: "translate".to_string(),
            description: "Translate text to another language",
            parameters: translate_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("".into()) })),
        },
    ];

    let inv_server = crate::test_mcp_server::spawn(
        "test",
        invention_tools.into_iter()
            .map(crate::test_mcp_server::TestTool::from_invention)
            .collect(),
    ).await;
    let conn = crate::test_mcp_server::connect_through_proxy(&[&inv_server]).await;
    let mut result = build_params(
        &agent,
        &params,
        &messages,
        None,
        Some(&conn),
    ).await;

    // Sort tools by name for deterministic comparison.
    if let Some(tools) = result.tools.as_mut() {
        tools.sort_by(|a, b| {
            let name_a = match a {
                Tool::Function { function } => &function.name,
            };
            let name_b = match b {
                Tool::Function { function } => &function.name,
            };
            name_a.cmp(name_b)
        });
    }

    let expected = ChatCompletionCreateParams {
        messages: messages.clone(),
        provider: None,
        model: "openai/gpt-4o".into(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: Some(super::tool_choice::ToolChoice::Auto),
        tools: Some(vec![
            Tool::Function {
                function: FunctionTool {
                    name: "test_calculate".into(),
                    description: Some("Evaluate a math expression".into()),
                    parameters: Some(calculate_params),
                    strict: None,
                },
            },
            Tool::Function {
                function: FunctionTool {
                    name: "test_search".into(),
                    description: Some("Search the web".into()),
                    parameters: Some(search_params),
                    strict: None,
                },
            },
            Tool::Function {
                function: FunctionTool {
                    name: "test_translate".into(),
                    description: Some("Translate text to another language".into()),
                    parameters: Some(translate_params),
                    strict: None,
                },
            },
        ]),
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: StreamOptions {
            include_usage: Some(true),
        },
        usage: Usage { include: true },
    };

    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_toolcall_not_required_uses_auto_choice() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: Some(
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::Single(
                objectiveai_sdk::agent::completions::request::ResponseFormat::ToolCall {
                    name: "summarize".into(),
                    description: "Summarize text".into(),
                    schema: {
                        let mut m = indexmap::IndexMap::new();
                        m.insert(
                            "type".to_string(),
                            serde_json::Value::String("object".to_string()),
                        );
                        m
                    },
                    required: None,
                },
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages: Vec<objectiveai_sdk::agent::completions::message::Message> = vec![];
    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        None,
    ).await;

    let expected = ChatCompletionCreateParams {
        messages: vec![],
        provider: None,
        model: "openai/gpt-4o".into(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: Some(super::tool_choice::ToolChoice::Auto),
        tools: Some(vec![super::Tool::Function {
            function: super::FunctionTool {
                name: "summarize".into(),
                description: Some("Summarize text".into()),
                parameters: Some({
                    let mut m = indexmap::IndexMap::new();
                    m.insert(
                        "type".to_string(),
                        serde_json::Value::String("object".to_string()),
                    );
                    m
                }),
                strict: None,
            },
        }]),
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: super::StreamOptions {
            include_usage: Some(true),
        },
        usage: super::Usage { include: true },
    };

    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_invention_tool_parameters_preserved() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai_sdk::agent::completions::message::Message::User(
            objectiveai_sdk::agent::completions::message::UserMessage {
                content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                    "Hello".into(),
                ),
                name: None,
            },
        ),
    ];

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let mut inv_params = indexmap::IndexMap::new();
    inv_params.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    inv_params.insert(
        "properties".to_string(),
        serde_json::json!({
            "query": {"type": "string", "description": "The search query"},
            "limit": {"type": "integer", "description": "Max results"}
        }),
    );
    inv_params.insert(
        "required".to_string(),
        serde_json::Value::Array(vec![
            serde_json::Value::String("query".to_string()),
        ]),
    );
    inv_params.insert(
        "additionalProperties".to_string(),
        serde_json::Value::Bool(false),
    );

    let invention_tools = vec![
        objectiveai_sdk::functions::inventions::InventionTool {
            name: "analyze".to_string(),
            description: "Analyze data",
            parameters: inv_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("ok".into()) })),
        },
    ];

    let inv_server = crate::test_mcp_server::spawn(
        "test",
        invention_tools.into_iter()
            .map(crate::test_mcp_server::TestTool::from_invention)
            .collect(),
    ).await;
    let conn = crate::test_mcp_server::connect_through_proxy(&[&inv_server]).await;
    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        Some(&conn),
    ).await;

    let expected = ChatCompletionCreateParams {
        messages: messages.clone(),
        provider: None,
        model: "test-model".into(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: Some(super::tool_choice::ToolChoice::Auto),
        tools: Some(vec![
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "test_analyze".to_string(),
                    description: Some("Analyze data".to_string()),
                    parameters: Some(inv_params),
                    strict: None,
                },
            },
        ]),
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: super::StreamOptions {
            include_usage: Some(true),
        },
        usage: super::Usage { include: true },
    };

    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_agent_base_fields_passthrough() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".to_string(),
            temperature: Some(0.7),
            top_p: Some(0.9),
            frequency_penalty: Some(0.5),
            presence_penalty: Some(-0.3),
            max_completion_tokens: Some(4096),
            max_tokens: Some(2048),
            min_p: Some(0.05),
            top_k: Some(50),
            top_a: Some(0.1),
            repetition_penalty: Some(1.1),
            top_logprobs: Some(5),
            stop: Some(objectiveai_sdk::agent::openrouter::Stop::Strings(vec![
                "END".into(),
                "STOP".into(),
            ])),
            verbosity: Some(objectiveai_sdk::agent::openrouter::Verbosity::High),
            ..Default::default()
        },
    )
    .unwrap();

    let params =
        objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
            messages: vec![],
            provider: None,
            agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                    inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                    fallbacks: None,
                },
            ),
            response_format: None,
            seed: None,
            stream: None,
            continuation: None,
            };

    let messages = vec![
        objectiveai_sdk::agent::completions::message::Message::User(
            objectiveai_sdk::agent::completions::message::UserMessage {
                content:
                    objectiveai_sdk::agent::completions::message::RichContent::Text(
                        "Hello".to_string(),
                    ),
                name: None,
            },
        ),
    ];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        None,
    ).await;

    assert_eq!(
        result,
        ChatCompletionCreateParams {
            messages: messages.clone(),
            provider: None,
            model: "openai/gpt-4o".to_string(),
            frequency_penalty: Some(0.5),
            logit_bias: None,
            max_completion_tokens: Some(4096),
            presence_penalty: Some(-0.3),
            stop: Some(objectiveai_sdk::agent::openrouter::Stop::Strings(vec![
                "END".into(),
                "STOP".into(),
            ])),
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(2048),
            min_p: Some(0.05),
            reasoning: None,
            repetition_penalty: Some(1.1),
            top_a: Some(0.1),
            top_k: Some(50),
            verbosity: Some(objectiveai_sdk::agent::openrouter::Verbosity::High),
            logprobs: Some(true),
            top_logprobs: Some(5),
            response_format: None,
            seed: None,
            tool_choice: None,
            tools: None,
            parallel_tool_calls: None,
            prediction: None,
            stream: true,
            stream_options: StreamOptions {
                include_usage: Some(true),
            },
            usage: Usage { include: true },
        }
    );
}

#[tokio::test]
async fn test_provider_merging_both_sides() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            provider: Some(objectiveai_sdk::agent::openrouter::Provider {
                allow_fallbacks: Some(false),
                require_parameters: Some(true),
                order: Some(vec!["anthropic".into()]),
                only: None,
                ignore: None,
                quantizations: None,
            }),
            ..Default::default()
        },
    )
    .unwrap();

    let params =
        objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
            messages: vec![],
            provider: Some(
                objectiveai_sdk::agent::completions::request::Provider {
                    data_collection: Some(
                        objectiveai_sdk::agent::completions::request::ProviderDataCollection::Deny,
                    ),
                    zdr: Some(true),
                    sort: Some(
                        objectiveai_sdk::agent::completions::request::ProviderSort::Price,
                    ),
                    max_price: None,
                    preferred_min_throughput: Some(100.0),
                    preferred_max_latency: None,
                    min_throughput: None,
                    max_latency: None,
                },
            ),
            agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                    inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                    fallbacks: None,
                },
            ),
            response_format: None,
            seed: None,
            stream: None,
            continuation: None,
            };

    let messages: Vec<objectiveai_sdk::agent::completions::message::Message> = vec![];
    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        None,
    ).await;

    let expected = ChatCompletionCreateParams {
        messages: vec![],
        provider: Some(super::provider::Provider {
            allow_fallbacks: Some(false),
            require_parameters: Some(true),
            data_collection: Some(
                objectiveai_sdk::agent::completions::request::ProviderDataCollection::Deny,
            ),
            zdr: Some(true),
            order: Some(vec!["anthropic".into()]),
            only: None,
            ignore: None,
            quantizations: None,
            sort: Some(
                objectiveai_sdk::agent::completions::request::ProviderSort::Price,
            ),
            max_price: None,
            preferred_min_throughput: Some(100.0),
            preferred_max_latency: None,
            min_throughput: None,
            max_latency: None,
        }),
        model: "openai/gpt-4o".into(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: None,
        tools: None,
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: StreamOptions {
            include_usage: Some(true),
        },
        usage: Usage { include: true },
    };

    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_per_agent_response_format_miss() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let mut per_agent_map = indexmap::IndexMap::new();
    per_agent_map.insert(
        "nonexistent_agent_id".to_string(),
        objectiveai_sdk::agent::completions::request::ResponseFormat::Text,
    );

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: Some(
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::PerAgent(
                per_agent_map,
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages: Vec<objectiveai_sdk::agent::completions::message::Message> = vec![];
    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        None,
    ).await;

    assert_eq!(
        result,
        ChatCompletionCreateParams {
            messages: vec![],
            provider: None,
            model: "gpt-4o".to_string(),
            frequency_penalty: None,
            logit_bias: None,
            max_completion_tokens: None,
            presence_penalty: None,
            stop: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            min_p: None,
            reasoning: None,
            repetition_penalty: None,
            top_a: None,
            top_k: None,
            verbosity: None,
            logprobs: None,
            top_logprobs: None,
            response_format: None,
            seed: None,
            tool_choice: None,
            tools: None,
            parallel_tool_calls: None,
            prediction: None,
            stream: true,
            stream_options: StreamOptions {
                include_usage: Some(true),
            },
            usage: Usage { include: true },
        }
    );
}

#[tokio::test]
async fn test_json_schema_response_format_extracts_title() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let mut schema = indexmap::IndexMap::new();
    schema.insert(
        "title".to_string(),
        serde_json::Value::String("MyResponse".to_string()),
    );
    schema.insert(
        "description".to_string(),
        serde_json::Value::String("A test schema".to_string()),
    );
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    schema.insert(
        "properties".to_string(),
        serde_json::json!({
            "name": { "type": "string" },
            "age": { "type": "integer" }
        }),
    );

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: Some(
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::Single(
                objectiveai_sdk::agent::completions::request::ResponseFormat::JsonSchema {
                    schema: schema.clone(),
                },
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages: Vec<objectiveai_sdk::agent::completions::message::Message> = vec![];
    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        None,
    ).await;

    let mut expected_schema_map = serde_json::Map::new();
    expected_schema_map.insert(
        "title".to_string(),
        serde_json::Value::String("MyResponse".to_string()),
    );
    expected_schema_map.insert(
        "description".to_string(),
        serde_json::Value::String("A test schema".to_string()),
    );
    expected_schema_map.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    expected_schema_map.insert(
        "properties".to_string(),
        serde_json::json!({
            "name": { "type": "string" },
            "age": { "type": "integer" }
        }),
    );

    assert_eq!(
        result,
        ChatCompletionCreateParams {
            messages: vec![],
            provider: None,
            model: "openai/gpt-4o".to_string(),
            frequency_penalty: None,
            logit_bias: None,
            max_completion_tokens: None,
            presence_penalty: None,
            stop: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            min_p: None,
            reasoning: None,
            repetition_penalty: None,
            top_a: None,
            top_k: None,
            verbosity: None,
            logprobs: None,
            top_logprobs: None,
            response_format: Some(super::response_format::ResponseFormat::JsonSchema {
                json_schema: super::response_format::JsonSchema {
                    name: "MyResponse".to_string(),
                    description: Some("A test schema".to_string()),
                    schema: Some(serde_json::Value::Object(expected_schema_map)),
                    strict: None,
                },
            }),
            seed: None,
            tool_choice: None,
            tools: None,
            parallel_tool_calls: None,
            prediction: None,
            stream: true,
            stream_options: StreamOptions {
                include_usage: Some(true),
            },
            usage: Usage { include: true },
        }
    );
}

#[tokio::test]
async fn test_seed_passthrough() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: Some(42),
        stream: None,
        continuation: None,
    };

    let messages = vec![
        objectiveai_sdk::agent::completions::message::Message::System(
            objectiveai_sdk::agent::completions::message::SystemMessage {
                content: objectiveai_sdk::agent::completions::message::SimpleContent::Text(
                    "You are a helpful assistant".into(),
                ),
                name: None,
            },
        ),
        objectiveai_sdk::agent::completions::message::Message::User(
            objectiveai_sdk::agent::completions::message::UserMessage {
                content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                    "What's the weather?".into(),
                ),
                name: None,
            },
        ),
        objectiveai_sdk::agent::completions::message::Message::Assistant(
            objectiveai_sdk::agent::completions::message::AssistantMessage {
                content: None,
                name: None,
                refusal: None,
                tool_calls: Some(vec![
                    objectiveai_sdk::agent::completions::message::AssistantToolCall::Function {
                        id: "call_1".into(),
                        function: objectiveai_sdk::agent::completions::message::AssistantToolCallFunction {
                            name: "get_weather".into(),
                            arguments: "{\"city\":\"SF\"}".into(),
                        },
                    },
                ]),
                reasoning: None,
            },
        ),
        objectiveai_sdk::agent::completions::message::Message::Tool(
            objectiveai_sdk::agent::completions::message::ToolMessage {
                content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                    "Sunny, 72F".into(),
                ),
                tool_call_id: "call_1".into(),
            },
        ),
    ];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        None,
    ).await;

    assert_eq!(
        result,
        ChatCompletionCreateParams {
            messages: messages.clone(),
            provider: None,
            model: "openai/gpt-4o".into(),
            frequency_penalty: None,
            logit_bias: None,
            max_completion_tokens: None,
            presence_penalty: None,
            stop: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            min_p: None,
            reasoning: None,
            repetition_penalty: None,
            top_a: None,
            top_k: None,
            verbosity: None,
            logprobs: None,
            top_logprobs: None,
            response_format: None,
            seed: Some(42),
            tool_choice: None,
            tools: None,
            parallel_tool_calls: None,
            prediction: None,
            stream: true,
            stream_options: StreamOptions {
                include_usage: Some(true),
            },
            usage: Usage { include: true },
        }
    );
}

#[tokio::test]
async fn test_toolcall_required_forces_function_choice() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let mut schema = indexmap::IndexMap::new();
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    schema.insert(
        "properties".to_string(),
        serde_json::json!({
            "score": { "type": "number", "description": "A score from 0 to 1" },
            "reasoning": { "type": "string", "description": "Explanation for the score" }
        }),
    );
    schema.insert(
        "required".to_string(),
        serde_json::json!(["score", "reasoning"]),
    );

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: Some(
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::Single(
                objectiveai_sdk::agent::completions::request::ResponseFormat::ToolCall {
                    name: "evaluate".into(),
                    description: "Evaluate the input".into(),
                    schema: schema.clone(),
                    required: Some(true),
                },
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    let result = build_params(
        &agent,
        &params,
        &[],
        None,
        None,
    ).await;

    let expected = ChatCompletionCreateParams {
        messages: vec![],
        provider: None,
        model: "openai/gpt-4o".into(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: Some(super::tool_choice::ToolChoice::Function(
            super::tool_choice::ToolChoiceFunction::Function {
                function: super::tool_choice::ToolChoiceFunctionFunction {
                    name: "evaluate".into(),
                },
            },
        )),
        tools: Some(vec![super::Tool::Function {
            function: super::FunctionTool {
                name: "evaluate".into(),
                description: Some("Evaluate the input".into()),
                parameters: Some(schema),
                strict: None,
            },
        }]),
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: super::StreamOptions {
            include_usage: Some(true),
        },
        usage: super::Usage { include: true },
    };

    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_three_mcp_servers_fifteen_tools_all_unique() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "anthropic/claude-sonnet-4".into(),
            temperature: Some(0.3),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai_sdk::agent::completions::message::Message::User(
            objectiveai_sdk::agent::completions::message::UserMessage {
                content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                    "Use the tools".into(),
                ),
                name: None,
            },
        ),
    ];

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    // Server 1: file operations
    let tools1 = vec![
        objectiveai_sdk::mcp::tool::Tool {
            name: "read_file".into(),
            title: None,
            description: Some("Read a file from disk".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "path".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["path".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "write_file".into(),
            title: None,
            description: Some("Write content to a file".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "path".into() => serde_json::json!({"type": "string"}),
                    "content".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["path".into(), "content".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "list_dir".into(),
            title: Some("List Directory".into()),
            description: Some("List files in a directory".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "path".into() => serde_json::json!({"type": "string"}),
                    "recursive".into() => serde_json::json!({"type": "boolean", "default": false}),
                }),
                required: Some(vec!["path".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "delete_file".into(),
            title: None,
            description: None,
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "path".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["path".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "file_info".into(),
            title: None,
            description: Some("Get file metadata".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "path".into() => serde_json::json!({"type": "string"}),
                }),
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ];

    // Server 2: database operations
    let tools2 = vec![
        objectiveai_sdk::mcp::tool::Tool {
            name: "query".into(),
            title: None,
            description: Some("Run a SQL query".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "sql".into() => serde_json::json!({"type": "string"}),
                    "database".into() => serde_json::json!({"type": "string", "enum": ["prod", "staging"]}),
                }),
                required: Some(vec!["sql".into()]),
                extra: indexmap::indexmap! {
                    "additionalProperties".into() => serde_json::Value::Bool(false),
                },
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "insert".into(),
            title: None,
            description: Some("Insert a row".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "table".into() => serde_json::json!({"type": "string"}),
                    "data".into() => serde_json::json!({"type": "object"}),
                }),
                required: Some(vec!["table".into(), "data".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "update".into(),
            title: None,
            description: Some("Update rows".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "table".into() => serde_json::json!({"type": "string"}),
                    "set".into() => serde_json::json!({"type": "object"}),
                    "where".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["table".into(), "set".into(), "where".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "delete".into(),
            title: None,
            description: Some("Delete rows".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "table".into() => serde_json::json!({"type": "string"}),
                    "where".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["table".into(), "where".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "list_tables".into(),
            title: None,
            description: Some("List all tables".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: None,
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ];

    // Server 3: web/HTTP operations
    let tools3 = vec![
        objectiveai_sdk::mcp::tool::Tool {
            name: "fetch_url".into(),
            title: None,
            description: Some("Fetch a URL".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "url".into() => serde_json::json!({"type": "string", "format": "uri"}),
                    "method".into() => serde_json::json!({"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"]}),
                    "headers".into() => serde_json::json!({"type": "object"}),
                }),
                required: Some(vec!["url".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "parse_html".into(),
            title: None,
            description: Some("Parse HTML and extract text".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "html".into() => serde_json::json!({"type": "string"}),
                    "selector".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["html".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "screenshot".into(),
            title: None,
            description: Some("Take a screenshot of a webpage".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "url".into() => serde_json::json!({"type": "string"}),
                    "width".into() => serde_json::json!({"type": "integer", "default": 1280}),
                    "height".into() => serde_json::json!({"type": "integer", "default": 720}),
                }),
                required: Some(vec!["url".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "dns_lookup".into(),
            title: None,
            description: Some("DNS lookup".into()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "hostname".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["hostname".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai_sdk::mcp::tool::Tool {
            name: "whois".into(),
            title: None,
            description: None,
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "domain".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["domain".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ];

    let server1 = crate::test_mcp_server::spawn(
        "fs",
        tools1.into_iter().map(crate::test_mcp_server::TestTool::noop).collect(),
    ).await;
    let server2 = crate::test_mcp_server::spawn(
        "db",
        tools2.into_iter().map(crate::test_mcp_server::TestTool::noop).collect(),
    ).await;
    let server3 = crate::test_mcp_server::spawn(
        "web",
        tools3.into_iter().map(crate::test_mcp_server::TestTool::noop).collect(),
    ).await;
    let conn = crate::test_mcp_server::connect_through_proxy(&[&server1, &server2, &server3]).await;
    let mut result = build_params(
        &agent,
        &params,
        &messages,
        None,
        Some(&conn),
    ).await;

    if let Some(tools) = result.tools.as_mut() {
        tools.sort_by(|a, b| {
            let name_a = match a { Tool::Function { function } => &function.name };
            let name_b = match b { Tool::Function { function } => &function.name };
            name_a.cmp(name_b)
        });
    }

    let expected = ChatCompletionCreateParams {
        messages: messages.clone(),
        provider: None,
        model: "anthropic/claude-sonnet-4".into(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: Some(0.3),
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: Some(super::tool_choice::ToolChoice::Auto),
        tools: Some(vec![
            super::Tool::Function { function: super::FunctionTool {
                name: "db_delete".into(),
                description: Some("Delete rows".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("table".into(), serde_json::json!({"type": "string"})),
                        ("where".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["table", "where"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "db_insert".into(),
                description: Some("Insert a row".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("table".into(), serde_json::json!({"type": "string"})),
                        ("data".into(), serde_json::json!({"type": "object"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["table", "data"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "db_list_tables".into(),
                description: Some("List all tables".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "db_query".into(),
                description: Some("Run a SQL query".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("sql".into(), serde_json::json!({"type": "string"})),
                        ("database".into(), serde_json::json!({"type": "string", "enum": ["prod", "staging"]})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["sql"]),
                    "additionalProperties".into() => serde_json::Value::Bool(false),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "db_update".into(),
                description: Some("Update rows".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("table".into(), serde_json::json!({"type": "string"})),
                        ("set".into(), serde_json::json!({"type": "object"})),
                        ("where".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["table", "set", "where"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "fs_delete_file".into(),
                description: None,
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("path".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["path"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "fs_file_info".into(),
                description: Some("Get file metadata".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("path".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "fs_list_dir".into(),
                description: Some("List files in a directory".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("path".into(), serde_json::json!({"type": "string"})),
                        ("recursive".into(), serde_json::json!({"type": "boolean", "default": false})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["path"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "fs_read_file".into(),
                description: Some("Read a file from disk".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("path".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["path"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "fs_write_file".into(),
                description: Some("Write content to a file".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("path".into(), serde_json::json!({"type": "string"})),
                        ("content".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["path", "content"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "web_dns_lookup".into(),
                description: Some("DNS lookup".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("hostname".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["hostname"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "web_fetch_url".into(),
                description: Some("Fetch a URL".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("url".into(), serde_json::json!({"type": "string", "format": "uri"})),
                        ("method".into(), serde_json::json!({"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"]})),
                        ("headers".into(), serde_json::json!({"type": "object"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["url"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "web_parse_html".into(),
                description: Some("Parse HTML and extract text".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("html".into(), serde_json::json!({"type": "string"})),
                        ("selector".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["html"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "web_screenshot".into(),
                description: Some("Take a screenshot of a webpage".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("url".into(), serde_json::json!({"type": "string"})),
                        ("width".into(), serde_json::json!({"type": "integer", "default": 1280})),
                        ("height".into(), serde_json::json!({"type": "integer", "default": 720})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["url"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "web_whois".into(),
                description: None,
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("domain".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["domain"]),
                }),
                strict: None,
            }},
        ]),
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: super::StreamOptions {
            include_usage: Some(true),
        },
        usage: super::Usage { include: true },
    };

    assert_eq!(result, expected);
}





#[tokio::test]
async fn test_continuation_assistant_message_appended() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent {
        id: String::new(),
        base: objectiveai_sdk::agent::openrouter::AgentBase {
            model: "test-model".to_string(),
            ..Default::default()
        },
    };

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages = vec![objectiveai_sdk::agent::completions::message::Message::User(
        objectiveai_sdk::agent::completions::message::UserMessage {
            content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                "Hello".to_string(),
            ),
            name: None,
        },
    )];

    let continuation = vec![
        crate::agent::completions::ContinuationItem::State(
            objectiveai_sdk::agent::completions::message::AssistantMessage {
                content: Some(objectiveai_sdk::agent::completions::message::RichContent::Text(
                    "Hi there!".to_string(),
                )),
                name: None,
                refusal: None,
                tool_calls: None,
                reasoning: None,
            },
        ),
    ];

    let result = build_params(
        &agent,
        &params,
        &messages,
        Some(&continuation),
        None,
    ).await;

    let expected = ChatCompletionCreateParams {
        messages: vec![
            objectiveai_sdk::agent::completions::message::Message::User(
                objectiveai_sdk::agent::completions::message::UserMessage {
                    content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                        "Hello".to_string(),
                    ),
                    name: None,
                },
            ),
            objectiveai_sdk::agent::completions::message::Message::Assistant(
                objectiveai_sdk::agent::completions::message::AssistantMessage {
                    content: Some(
                        objectiveai_sdk::agent::completions::message::RichContent::Text(
                            "Hi there!".to_string(),
                        ),
                    ),
                    name: None,
                    refusal: None,
                    tool_calls: None,
                    reasoning: None,
                },
            ),
        ],
        provider: None,
        model: "test-model".to_string(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: None,
        tools: None,
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: super::StreamOptions {
            include_usage: Some(true),
        },
        usage: super::Usage { include: true },
    };

    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_continuation_mixed_items() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent {
        id: String::new(),
        base: objectiveai_sdk::agent::openrouter::AgentBase {
            model: "test-model".to_string(),
            ..Default::default()
        },
    };

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages = vec![objectiveai_sdk::agent::completions::message::Message::User(
        objectiveai_sdk::agent::completions::message::UserMessage {
            content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                "What is the weather?".to_string(),
            ),
            name: None,
        },
    )];

    let continuation = vec![
        // Assistant made a tool call
        crate::agent::completions::ContinuationItem::State(
            objectiveai_sdk::agent::completions::message::AssistantMessage {
                content: None,
                name: None,
                refusal: None,
                tool_calls: Some(vec![
                    objectiveai_sdk::agent::completions::message::AssistantToolCall::Function {
                        id: "call_abc".to_string(),
                        function:
                            objectiveai_sdk::agent::completions::message::AssistantToolCallFunction {
                                name: "get_weather".to_string(),
                                arguments: "{\"city\":\"NYC\"}".to_string(),
                            },
                    },
                ]),
                reasoning: None,
            },
        ),
        // Tool response
        crate::agent::completions::ContinuationItem::ToolMessage(
            objectiveai_sdk::agent::completions::message::ToolMessage {
                content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                    "Sunny, 72F".to_string(),
                ),
                tool_call_id: "call_abc".to_string(),
            },
        ),
        // User follow-up
        crate::agent::completions::ContinuationItem::UserMessage(
            objectiveai_sdk::agent::completions::message::UserMessage {
                content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                    "Thanks! What about tomorrow?".to_string(),
                ),
                name: None,
            },
        ),
    ];

    let result = build_params(
        &agent,
        &params,
        &messages,
        Some(&continuation),
        None,
    ).await;

    let expected = ChatCompletionCreateParams {
        messages: vec![
            objectiveai_sdk::agent::completions::message::Message::User(
                objectiveai_sdk::agent::completions::message::UserMessage {
                    content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                        "What is the weather?".to_string(),
                    ),
                    name: None,
                },
            ),
            objectiveai_sdk::agent::completions::message::Message::Assistant(
                objectiveai_sdk::agent::completions::message::AssistantMessage {
                    content: None,
                    name: None,
                    refusal: None,
                    tool_calls: Some(vec![
                        objectiveai_sdk::agent::completions::message::AssistantToolCall::Function {
                            id: "call_abc".to_string(),
                            function:
                                objectiveai_sdk::agent::completions::message::AssistantToolCallFunction {
                                    name: "get_weather".to_string(),
                                    arguments: "{\"city\":\"NYC\"}".to_string(),
                                },
                        },
                    ]),
                    reasoning: None,
                },
            ),
            objectiveai_sdk::agent::completions::message::Message::Tool(
                objectiveai_sdk::agent::completions::message::ToolMessage {
                    content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                        "Sunny, 72F".to_string(),
                    ),
                    tool_call_id: "call_abc".to_string(),
                },
            ),
            objectiveai_sdk::agent::completions::message::Message::User(
                objectiveai_sdk::agent::completions::message::UserMessage {
                    content: objectiveai_sdk::agent::completions::message::RichContent::Text(
                        "Thanks! What about tomorrow?".to_string(),
                    ),
                    name: None,
                },
            ),
        ],
        provider: None,
        model: "test-model".to_string(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        min_p: None,
        reasoning: None,
        repetition_penalty: None,
        top_a: None,
        top_k: None,
        verbosity: None,
        logprobs: None,
        top_logprobs: None,
        response_format: None,
        seed: None,
        tool_choice: None,
        tools: None,
        parallel_tool_calls: None,
        prediction: None,
        stream: true,
        stream_options: super::StreamOptions {
            include_usage: Some(true),
        },
        usage: super::Usage { include: true },
    };

    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_tools_disabled_sets_tool_choice_none() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    // Use an optional ToolCall response format so tools get resolved but
    // it's not required (required would be rejected earlier in the client).
    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: Some(
            objectiveai_sdk::agent::completions::request::ResponseFormatParam::Single(
                objectiveai_sdk::agent::completions::request::ResponseFormat::ToolCall {
                    name: "my_tool".into(),
                    description: "a tool".into(),
                    schema: indexmap::IndexMap::new(),
                    required: None,
                },
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    let result = build_params_with_tools_enabled(
        &agent, &params, &[], None, None, false,
    ).await;

    // tool_choice should be None (the enum variant meaning "none"),
    // not Auto or Function.
    assert_eq!(
        result.tool_choice,
        Some(super::tool_choice::ToolChoice::None),
        "tools_enabled=false should set tool_choice to none",
    );
    // Tools should still be present in the request.
    assert!(
        result.tools.is_some(),
        "tools should still be included when tools_enabled=false",
    );
}

#[tokio::test]
async fn test_tools_disabled_no_tools_no_tool_choice() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let result = build_params_with_tools_enabled(
        &agent, &params, &[], None, None, false,
    ).await;

    // No tools means no tool_choice at all.
    assert_eq!(result.tool_choice, None);
    assert!(result.tools.is_none());
}

#[tokio::test]
async fn test_request_continuation_messages_come_first() {
    let _permit = crate::test_clients::acquire_test_permit().await;
    use objectiveai_sdk::agent::completions::message::*;

    let agent = objectiveai_sdk::agent::openrouter::Agent::try_from(
        objectiveai_sdk::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Current turn".into()),
        name: None,
    })];

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let request_continuation = objectiveai_sdk::agent::openrouter::Continuation {
        upstream: objectiveai_sdk::agent::openrouter::Upstream::default(),
        messages: vec![
            Message::User(UserMessage {
                content: RichContent::Text("Previous turn".into()),
                name: None,
            }),
            Message::Assistant(AssistantMessage {
                content: Some(RichContent::Text("Previous response".into())),
                name: None,
                refusal: None,
                tool_calls: None,
                reasoning: None,
            }),
        ],
        mcp_sessions: indexmap::IndexMap::new(),
    };

    let result = ChatCompletionCreateParams::new(
        &agent, &params, &messages, None, Some(&request_continuation),
        &[], &std::collections::HashMap::new(), true,
    );

    // Request continuation messages come first, then argument messages.
    assert_eq!(result.messages.len(), 3);
    // First: previous user message from continuation
    assert!(
        serde_json::to_string(&result.messages[0]).unwrap().contains("Previous turn"),
    );
    // Second: previous assistant response from continuation
    assert!(
        serde_json::to_string(&result.messages[1]).unwrap().contains("Previous response"),
    );
    // Third: current turn user message
    assert!(
        serde_json::to_string(&result.messages[2]).unwrap().contains("Current turn"),
    );
}
