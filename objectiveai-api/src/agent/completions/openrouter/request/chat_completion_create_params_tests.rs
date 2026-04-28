//! Tests for [`ChatCompletionCreateParams`] construction.

use super::*;
use std::sync::Arc;

/// Helper to resolve tools and build params, replacing the old `new_with_tools`.
fn build_params(
    agent: &objectiveai::agent::openrouter::Agent,
    params: &objectiveai::agent::completions::request::AgentCompletionCreateParams,
    messages: &[objectiveai::agent::completions::message::Message],
    continuation: Option<&[crate::agent::completions::ContinuationItem<objectiveai::agent::completions::message::AssistantMessage>]>,
    mcp_connections: &[Arc<objectiveai::mcp::Connection>],
    mcp_tools: &[Arc<Vec<objectiveai::mcp::tool::Tool>>],
    invention_tools: Option<&[objectiveai::functions::inventions::InventionTool]>,
) -> ChatCompletionCreateParams {
    let resolved_rf = params.response_format.as_ref().and_then(|rfp| {
        match rfp {
            objectiveai::agent::completions::request::ResponseFormatParam::Single(rf) => Some(rf.clone()),
            objectiveai::agent::completions::request::ResponseFormatParam::PerAgent(map) => map.get(&agent.id).cloned(),
        }
    });
    let (tool_names, tool_map) = crate::agent::completions::tool::resolve_tools(
        mcp_connections,
        mcp_tools,
        invention_tools,
        resolved_rf.as_ref(),
    );
    ChatCompletionCreateParams::new(
        agent, params, messages, continuation, None,
        &tool_names, &tool_map, true,
    )
}

/// Like `build_params` but with explicit `tools_enabled` control.
fn build_params_with_tools_enabled(
    agent: &objectiveai::agent::openrouter::Agent,
    params: &objectiveai::agent::completions::request::AgentCompletionCreateParams,
    messages: &[objectiveai::agent::completions::message::Message],
    continuation: Option<&[crate::agent::completions::ContinuationItem<objectiveai::agent::completions::message::AssistantMessage>]>,
    mcp_connections: &[Arc<objectiveai::mcp::Connection>],
    mcp_tools: &[Arc<Vec<objectiveai::mcp::tool::Tool>>],
    invention_tools: Option<&[objectiveai::functions::inventions::InventionTool]>,
    tools_enabled: bool,
) -> ChatCompletionCreateParams {
    let resolved_rf = params.response_format.as_ref().and_then(|rfp| {
        match rfp {
            objectiveai::agent::completions::request::ResponseFormatParam::Single(rf) => Some(rf.clone()),
            objectiveai::agent::completions::request::ResponseFormatParam::PerAgent(map) => map.get(&agent.id).cloned(),
        }
    });
    let (tool_names, tool_map) = crate::agent::completions::tool::resolve_tools(
        mcp_connections,
        mcp_tools,
        invention_tools,
        resolved_rf.as_ref(),
    );
    ChatCompletionCreateParams::new(
        agent, params, messages, continuation, None,
        &tool_names, &tool_map, tools_enabled,
    )
}

#[test]
fn test_no_tools_empty_params() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Hello".into(),
                ),
                name: None,
            },
        ),
    ];

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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

#[test]
fn test_invention_response_format_name_conflict() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Hello".into(),
                ),
                name: None,
            },
        ),
    ];

    let mut rf_schema = indexmap::IndexMap::new();
    rf_schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    rf_schema.insert(
        "properties".to_string(),
        serde_json::json!({"result": {"type": "string"}}),
    );

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: Some(
            objectiveai::agent::completions::request::ResponseFormatParam::Single(
                objectiveai::agent::completions::request::ResponseFormat::ToolCall {
                    name: "output".to_string(),
                    description: "Format output".to_string(),
                    schema: rf_schema.clone(),
                    required: Some(false),
                },
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    let mut inv_params = indexmap::IndexMap::new();
    inv_params.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );

    let invention_tools = vec![
        objectiveai::functions::inventions::InventionTool {
            name: "output",
            description: "Invention output",
            parameters: inv_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("".into()) })),
        },
    ];

    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let mut result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        Some(&invention_tools),
    );

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
                    name: "output".to_string(),
                    description: Some("Format output".to_string()),
                    parameters: Some(rf_schema),
                    strict: None,
                },
            },
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "output (objectiveai-invention)".to_string(),
                    description: Some("Invention output".to_string()),
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

#[test]
fn test_top_logprobs_zero_omits_logprobs() {
    let agent = objectiveai::agent::openrouter::Agent {
        id: String::new(),
        base: objectiveai::agent::openrouter::AgentBase {
            model: "test-model".to_string(),
            top_logprobs: Some(0),
            ..Default::default()
        },
    };

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages: Vec<objectiveai::agent::completions::message::Message> = vec![];
    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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

#[test]
fn test_multiple_invention_tools_no_conflicts() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages = vec![objectiveai::agent::completions::message::Message::User(
        objectiveai::agent::completions::message::UserMessage {
            content: objectiveai::agent::completions::message::RichContent::Text(
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
        objectiveai::functions::inventions::InventionTool {
            name: "search",
            description: "Search the web",
            parameters: search_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("".into()) })),
        },
        objectiveai::functions::inventions::InventionTool {
            name: "calculate",
            description: "Evaluate a math expression",
            parameters: calculate_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("".into()) })),
        },
        objectiveai::functions::inventions::InventionTool {
            name: "translate",
            description: "Translate text to another language",
            parameters: translate_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("".into()) })),
        },
    ];

    let mut result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &[],
        &[],
        Some(&invention_tools),
    );

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
                    name: "calculate".into(),
                    description: Some("Evaluate a math expression".into()),
                    parameters: Some(calculate_params),
                    strict: None,
                },
            },
            Tool::Function {
                function: FunctionTool {
                    name: "search".into(),
                    description: Some("Search the web".into()),
                    parameters: Some(search_params),
                    strict: None,
                },
            },
            Tool::Function {
                function: FunctionTool {
                    name: "translate".into(),
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

#[test]
fn test_toolcall_not_required_uses_auto_choice() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: Some(
            objectiveai::agent::completions::request::ResponseFormatParam::Single(
                objectiveai::agent::completions::request::ResponseFormat::ToolCall {
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

    let messages: Vec<objectiveai::agent::completions::message::Message> = vec![];
    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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

#[test]
fn test_invention_tool_parameters_preserved() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Hello".into(),
                ),
                name: None,
            },
        ),
    ];

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
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
        objectiveai::functions::inventions::InventionTool {
            name: "analyze",
            description: "Analyze data",
            parameters: inv_params.clone(),
            call: Arc::new(|_| Box::pin(async { Ok("ok".into()) })),
        },
    ];

    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        Some(&invention_tools),
    );

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
                    name: "analyze".to_string(),
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

#[test]
fn test_agent_base_fields_passthrough() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
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
            stop: Some(objectiveai::agent::openrouter::Stop::Strings(vec![
                "END".into(),
                "STOP".into(),
            ])),
            verbosity: Some(objectiveai::agent::openrouter::Verbosity::High),
            ..Default::default()
        },
    )
    .unwrap();

    let params =
        objectiveai::agent::completions::request::AgentCompletionCreateParams {
            messages: vec![],
            provider: None,
            agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                objectiveai::agent::InlineAgentBaseWithFallbacks {
                    inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                    fallbacks: None,
                },
            ),
            response_format: None,
            seed: None,
            stream: None,
            continuation: None,
            };

    let messages = vec![
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content:
                    objectiveai::agent::completions::message::RichContent::Text(
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
        &[],
        &[],
        None,
    );

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
            stop: Some(objectiveai::agent::openrouter::Stop::Strings(vec![
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
            verbosity: Some(objectiveai::agent::openrouter::Verbosity::High),
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

#[test]
fn test_provider_merging_both_sides() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            provider: Some(objectiveai::agent::openrouter::Provider {
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
        objectiveai::agent::completions::request::AgentCompletionCreateParams {
            messages: vec![],
            provider: Some(
                objectiveai::agent::completions::request::Provider {
                    data_collection: Some(
                        objectiveai::agent::completions::request::ProviderDataCollection::Deny,
                    ),
                    zdr: Some(true),
                    sort: Some(
                        objectiveai::agent::completions::request::ProviderSort::Price,
                    ),
                    max_price: None,
                    preferred_min_throughput: Some(100.0),
                    preferred_max_latency: None,
                    min_throughput: None,
                    max_latency: None,
                },
            ),
            agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                objectiveai::agent::InlineAgentBaseWithFallbacks {
                    inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                    fallbacks: None,
                },
            ),
            response_format: None,
            seed: None,
            stream: None,
            continuation: None,
            };

    let messages: Vec<objectiveai::agent::completions::message::Message> = vec![];
    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

    let expected = ChatCompletionCreateParams {
        messages: vec![],
        provider: Some(super::provider::Provider {
            allow_fallbacks: Some(false),
            require_parameters: Some(true),
            data_collection: Some(
                objectiveai::agent::completions::request::ProviderDataCollection::Deny,
            ),
            zdr: Some(true),
            order: Some(vec!["anthropic".into()]),
            only: None,
            ignore: None,
            quantizations: None,
            sort: Some(
                objectiveai::agent::completions::request::ProviderSort::Price,
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

#[test]
fn test_per_agent_response_format_miss() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let mut per_agent_map = indexmap::IndexMap::new();
    per_agent_map.insert(
        "nonexistent_agent_id".to_string(),
        objectiveai::agent::completions::request::ResponseFormat::Text,
    );

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: Some(
            objectiveai::agent::completions::request::ResponseFormatParam::PerAgent(
                per_agent_map,
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages: Vec<objectiveai::agent::completions::message::Message> = vec![];
    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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

#[test]
fn test_json_schema_response_format_extracts_title() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
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

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: Some(
            objectiveai::agent::completions::request::ResponseFormatParam::Single(
                objectiveai::agent::completions::request::ResponseFormat::JsonSchema {
                    schema: schema.clone(),
                },
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages: Vec<objectiveai::agent::completions::message::Message> = vec![];
    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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

#[test]
fn test_seed_passthrough() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: Some(42),
        stream: None,
        continuation: None,
    };

    let messages = vec![
        objectiveai::agent::completions::message::Message::System(
            objectiveai::agent::completions::message::SystemMessage {
                content: objectiveai::agent::completions::message::SimpleContent::Text(
                    "You are a helpful assistant".into(),
                ),
                name: None,
            },
        ),
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "What's the weather?".into(),
                ),
                name: None,
            },
        ),
        objectiveai::agent::completions::message::Message::Assistant(
            objectiveai::agent::completions::message::AssistantMessage {
                content: None,
                name: None,
                refusal: None,
                tool_calls: Some(vec![
                    objectiveai::agent::completions::message::AssistantToolCall::Function {
                        id: "call_1".into(),
                        function: objectiveai::agent::completions::message::AssistantToolCallFunction {
                            name: "get_weather".into(),
                            arguments: "{\"city\":\"SF\"}".into(),
                        },
                    },
                ]),
                reasoning: None,
            },
        ),
        objectiveai::agent::completions::message::Message::Tool(
            objectiveai::agent::completions::message::ToolMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
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
        &[],
        &[],
        None,
    );

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

#[test]
fn test_toolcall_required_forces_function_choice() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
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

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: Some(
            objectiveai::agent::completions::request::ResponseFormatParam::Single(
                objectiveai::agent::completions::request::ResponseFormat::ToolCall {
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

    let mcp_connections: Vec<Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &[],
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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

#[test]
fn test_three_mcp_servers_fifteen_tools_all_unique() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "anthropic/claude-sonnet-4".into(),
            temperature: Some(0.3),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Use the tools".into(),
                ),
                name: None,
            },
        ),
    ];

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
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
    let conn1 = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://files.example.com/mcp".into(),
    );
    let tools1 = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "read_file".into(),
            title: None,
            description: Some("Read a file from disk".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "write_file".into(),
            title: None,
            description: Some("Write content to a file".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "list_dir".into(),
            title: Some("List Directory".into()),
            description: Some("List files in a directory".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "delete_file".into(),
            title: None,
            description: None,
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "file_info".into(),
            title: None,
            description: Some("Get file metadata".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
    ]);

    // Server 2: database operations
    let conn2 = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://db.example.com/mcp".into(),
    );
    let tools2 = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "query".into(),
            title: None,
            description: Some("Run a SQL query".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "insert".into(),
            title: None,
            description: Some("Insert a row".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "update".into(),
            title: None,
            description: Some("Update rows".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "delete".into(),
            title: None,
            description: Some("Delete rows".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "list_tables".into(),
            title: None,
            description: Some("List all tables".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: None,
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ]);

    // Server 3: web/HTTP operations
    let conn3 = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://web.example.com/mcp".into(),
    );
    let tools3 = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "fetch_url".into(),
            title: None,
            description: Some("Fetch a URL".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "parse_html".into(),
            title: None,
            description: Some("Parse HTML and extract text".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "screenshot".into(),
            title: None,
            description: Some("Take a screenshot of a webpage".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "dns_lookup".into(),
            title: None,
            description: Some("DNS lookup".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
        objectiveai::mcp::tool::Tool {
            name: "whois".into(),
            title: None,
            description: None,
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
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
    ]);

    let mcp_connections = vec![conn1, conn2, conn3];
    let mcp_tools = vec![tools1, tools2, tools3];

    let mut result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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
                name: "delete".into(),
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
                name: "delete_file".into(),
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
                name: "dns_lookup".into(),
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
                name: "fetch_url".into(),
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
                name: "file_info".into(),
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
                name: "insert".into(),
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
                name: "list_dir".into(),
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
                name: "list_tables".into(),
                description: Some("List all tables".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "parse_html".into(),
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
                name: "query".into(),
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
                name: "read_file".into(),
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
                name: "screenshot".into(),
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
                name: "update".into(),
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
                name: "whois".into(),
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
            super::Tool::Function { function: super::FunctionTool {
                name: "write_file".into(),
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

#[test]
fn test_mcp_duplicate_name_across_servers_gets_url_suffix() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "google/gemini-2.5-pro".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai::agent::completions::message::Message::System(
            objectiveai::agent::completions::message::SystemMessage {
                content: objectiveai::agent::completions::message::SimpleContent::Text(
                    "You have access to multiple tool servers.".into(),
                ),
                name: None,
            },
        ),
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Search for something".into(),
                ),
                name: None,
            },
        ),
    ];

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: Some(7),
        stream: None,
        continuation: None,
    };

    // Server 1: knowledge base — has "search" (the duplicate)
    let conn1 = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://kb.example.com/mcp".into(),
    );
    let tools1 = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "search".into(),
            title: None,
            description: Some("Search the knowledge base".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "query".into() => serde_json::json!({"type": "string"}),
                    "top_k".into() => serde_json::json!({"type": "integer", "default": 10}),
                }),
                required: Some(vec!["query".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "index_document".into(),
            title: None,
            description: Some("Index a document".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "title".into() => serde_json::json!({"type": "string"}),
                    "body".into() => serde_json::json!({"type": "string"}),
                    "tags".into() => serde_json::json!({"type": "array", "items": {"type": "string"}}),
                }),
                required: Some(vec!["title".into(), "body".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "get_document".into(),
            title: None,
            description: Some("Retrieve a document by ID".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "id".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["id".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "delete_document".into(),
            title: None,
            description: Some("Delete a document".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "id".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["id".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "list_collections".into(),
            title: None,
            description: Some("List all collections".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: None,
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ]);

    // Server 2: code search — also has "search" (the duplicate!)
    let conn2 = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://code.example.com/mcp".into(),
    );
    let tools2 = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "search".into(),
            title: None,
            description: Some("Search code repositories".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "query".into() => serde_json::json!({"type": "string"}),
                    "language".into() => serde_json::json!({"type": "string"}),
                    "repo".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["query".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "get_file".into(),
            title: None,
            description: Some("Get file contents from repo".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "repo".into() => serde_json::json!({"type": "string"}),
                    "path".into() => serde_json::json!({"type": "string"}),
                    "ref".into() => serde_json::json!({"type": "string", "default": "main"}),
                }),
                required: Some(vec!["repo".into(), "path".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "list_repos".into(),
            title: None,
            description: Some("List repositories".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "org".into() => serde_json::json!({"type": "string"}),
                }),
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "blame".into(),
            title: None,
            description: Some("Git blame for a file".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "repo".into() => serde_json::json!({"type": "string"}),
                    "path".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["repo".into(), "path".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "diff".into(),
            title: None,
            description: Some("Diff between commits".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "repo".into() => serde_json::json!({"type": "string"}),
                    "base".into() => serde_json::json!({"type": "string"}),
                    "head".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["repo".into(), "base".into(), "head".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ]);

    // Server 3: email — no duplicates
    let conn3 = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://mail.example.com/mcp".into(),
    );
    let tools3 = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "send_email".into(),
            title: None,
            description: Some("Send an email".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "to".into() => serde_json::json!({"type": "string"}),
                    "subject".into() => serde_json::json!({"type": "string"}),
                    "body".into() => serde_json::json!({"type": "string"}),
                    "cc".into() => serde_json::json!({"type": "array", "items": {"type": "string"}}),
                }),
                required: Some(vec!["to".into(), "subject".into(), "body".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "read_inbox".into(),
            title: None,
            description: Some("Read inbox messages".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "limit".into() => serde_json::json!({"type": "integer", "default": 20}),
                    "unread_only".into() => serde_json::json!({"type": "boolean", "default": false}),
                }),
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "archive".into(),
            title: None,
            description: Some("Archive a message".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "message_id".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["message_id".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "create_draft".into(),
            title: None,
            description: Some("Create a draft email".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "to".into() => serde_json::json!({"type": "string"}),
                    "subject".into() => serde_json::json!({"type": "string"}),
                    "body".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["to".into(), "subject".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "list_labels".into(),
            title: None,
            description: Some("List email labels".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: None,
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ]);

    let mcp_connections = vec![conn1, conn2, conn3];
    let mcp_tools = vec![tools1, tools2, tools3];

    let mut result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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
        model: "google/gemini-2.5-pro".into(),
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
        seed: Some(7),
        tool_choice: Some(super::tool_choice::ToolChoice::Auto),
        tools: Some(vec![
            super::Tool::Function { function: super::FunctionTool {
                name: "archive".into(),
                description: Some("Archive a message".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("message_id".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["message_id"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "blame".into(),
                description: Some("Git blame for a file".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("repo".into(), serde_json::json!({"type": "string"})),
                        ("path".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["repo", "path"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "create_draft".into(),
                description: Some("Create a draft email".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("to".into(), serde_json::json!({"type": "string"})),
                        ("subject".into(), serde_json::json!({"type": "string"})),
                        ("body".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["to", "subject"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "delete_document".into(),
                description: Some("Delete a document".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("id".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["id"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "diff".into(),
                description: Some("Diff between commits".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("repo".into(), serde_json::json!({"type": "string"})),
                        ("base".into(), serde_json::json!({"type": "string"})),
                        ("head".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["repo", "base", "head"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "get_document".into(),
                description: Some("Retrieve a document by ID".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("id".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["id"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "get_file".into(),
                description: Some("Get file contents from repo".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("repo".into(), serde_json::json!({"type": "string"})),
                        ("path".into(), serde_json::json!({"type": "string"})),
                        ("ref".into(), serde_json::json!({"type": "string", "default": "main"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["repo", "path"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "index_document".into(),
                description: Some("Index a document".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("title".into(), serde_json::json!({"type": "string"})),
                        ("body".into(), serde_json::json!({"type": "string"})),
                        ("tags".into(), serde_json::json!({"type": "array", "items": {"type": "string"}})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["title", "body"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "list_collections".into(),
                description: Some("List all collections".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "list_labels".into(),
                description: Some("List email labels".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "list_repos".into(),
                description: Some("List repositories".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("org".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "read_inbox".into(),
                description: Some("Read inbox messages".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("limit".into(), serde_json::json!({"type": "integer", "default": 20})),
                        ("unread_only".into(), serde_json::json!({"type": "boolean", "default": false})),
                    ].into_iter().collect()),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "search (test(https://code.example.com/mcp))".into(),
                description: Some("Search code repositories".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("query".into(), serde_json::json!({"type": "string"})),
                        ("language".into(), serde_json::json!({"type": "string"})),
                        ("repo".into(), serde_json::json!({"type": "string"})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["query"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "search (test(https://kb.example.com/mcp))".into(),
                description: Some("Search the knowledge base".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("query".into(), serde_json::json!({"type": "string"})),
                        ("top_k".into(), serde_json::json!({"type": "integer", "default": 10})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["query"]),
                }),
                strict: None,
            }},
            super::Tool::Function { function: super::FunctionTool {
                name: "send_email".into(),
                description: Some("Send an email".into()),
                parameters: Some(indexmap::indexmap! {
                    "type".into() => serde_json::json!("object"),
                    "properties".into() => serde_json::Value::Object(vec![
                        ("to".into(), serde_json::json!({"type": "string"})),
                        ("subject".into(), serde_json::json!({"type": "string"})),
                        ("body".into(), serde_json::json!({"type": "string"})),
                        ("cc".into(), serde_json::json!({"type": "array", "items": {"type": "string"}})),
                    ].into_iter().collect()),
                    "required".into() => serde_json::json!(["to", "subject", "body"]),
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

#[test]
fn test_mcp_tool_conflicts_with_invention_tool() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Analyze this data".into(),
                ),
                name: None,
            },
        ),
    ];

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    // MCP server has a tool named "analyze"
    let conn = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://analytics.example.com/mcp".into(),
    );
    let mcp_tools_list = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "analyze".into(),
            title: None,
            description: Some("Run analytics query".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "dataset".into() => serde_json::json!({"type": "string"}),
                    "metric".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["dataset".into(), "metric".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "list_datasets".into(),
            title: None,
            description: Some("List available datasets".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: None,
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ]);

    // Invention tool also named "analyze"
    let invention_tools = vec![
        objectiveai::functions::inventions::InventionTool {
            name: "analyze",
            description: "Analyze with custom logic",
            parameters: indexmap::indexmap! {
                "type".into() => serde_json::json!("object"),
                "properties".into() => serde_json::json!({
                    "text": {"type": "string"},
                    "depth": {"type": "integer"}
                }),
                "required".into() => serde_json::json!(["text"]),
            },
            call: Arc::new(|_| Box::pin(async { Ok("done".into()) })),
        },
    ];

    let mcp_connections = vec![conn];
    let mcp_tools = vec![mcp_tools_list];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        Some(&invention_tools),
    );

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
            // MCP "analyze" gets server name suffix (MCP tools come first).
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "analyze (test)".into(),
                    description: Some("Run analytics query".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::Value::Object(
                            vec![
                                ("dataset".into(), serde_json::json!({"type": "string"})),
                                ("metric".into(), serde_json::json!({"type": "string"})),
                            ].into_iter().collect(),
                        ),
                        "required".into() => serde_json::json!(["dataset", "metric"]),
                    }),
                    strict: None,
                },
            },
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "list_datasets".into(),
                    description: Some("List available datasets".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                    }),
                    strict: None,
                },
            },
            // Invention "analyze" gets "(objectiveai-invention)" suffix (after MCP).
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "analyze (objectiveai-invention)".into(),
                    description: Some("Analyze with custom logic".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::json!({
                            "text": {"type": "string"},
                            "depth": {"type": "integer"}
                        }),
                        "required".into() => serde_json::json!(["text"]),
                    }),
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

#[test]
fn test_mcp_tool_conflicts_with_response_format_tool() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "openai/gpt-4o".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Evaluate this".into(),
                ),
                name: None,
            },
        ),
    ];

    // Response format ToolCall named "evaluate"
    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: Some(
            objectiveai::agent::completions::request::ResponseFormatParam::Single(
                objectiveai::agent::completions::request::ResponseFormat::ToolCall {
                    name: "evaluate".into(),
                    description: "Return evaluation scores".into(),
                    schema: indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::json!({
                            "score": {"type": "number"},
                            "confidence": {"type": "number"}
                        }),
                    },
                    required: Some(true),
                },
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    // MCP server also has a tool named "evaluate"
    let conn = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://grading.example.com/mcp".into(),
    );
    let mcp_tools_list = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "evaluate".into(),
            title: None,
            description: Some("Grade a student submission".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "submission_id".into() => serde_json::json!({"type": "string"}),
                    "rubric".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["submission_id".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "list_submissions".into(),
            title: None,
            description: Some("List student submissions".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "course_id".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["course_id".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ]);

    let mcp_connections = vec![conn];
    let mcp_tools = vec![mcp_tools_list];

    let mut result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        None,
    );

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
        tools: Some(vec![
            // RF "evaluate" keeps plain name.
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "evaluate".into(),
                    description: Some("Return evaluation scores".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::json!({
                            "score": {"type": "number"},
                            "confidence": {"type": "number"}
                        }),
                    }),
                    strict: None,
                },
            },
            // MCP "evaluate" gets URL suffix.
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "evaluate (test)".into(),
                    description: Some("Grade a student submission".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::Value::Object(
                            vec![
                                ("submission_id".into(), serde_json::json!({"type": "string"})),
                                ("rubric".into(), serde_json::json!({"type": "string"})),
                            ].into_iter().collect(),
                        ),
                        "required".into() => serde_json::json!(["submission_id"]),
                    }),
                    strict: None,
                },
            },
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "list_submissions".into(),
                    description: Some("List student submissions".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::Value::Object(
                            vec![
                                ("course_id".into(), serde_json::json!({"type": "string"})),
                            ].into_iter().collect(),
                        ),
                        "required".into() => serde_json::json!(["course_id"]),
                    }),
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

#[test]
fn test_four_way_name_conflict_mcp_x2_invention_response_format() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "anthropic/claude-sonnet-4".into(),
            temperature: Some(0.5),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![
        objectiveai::agent::completions::message::Message::User(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Generate output".into(),
                ),
                name: None,
            },
        ),
    ];

    // Response format ToolCall named "output" (required=false → Auto)
    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: Some(
            objectiveai::agent::completions::request::ResponseFormatParam::Single(
                objectiveai::agent::completions::request::ResponseFormat::ToolCall {
                    name: "output".into(),
                    description: "Structured output from RF".into(),
                    schema: indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::json!({
                            "result": {"type": "string"}
                        }),
                    },
                    required: None,
                },
            ),
        ),
        seed: None,
        stream: None,
        continuation: None,
    };

    // MCP server 1 has "output"
    let conn1 = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://renderer.example.com/mcp".into(),
    );
    let mcp_tools1 = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "output".into(),
            title: None,
            description: Some("Render output to display".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "format".into() => serde_json::json!({"type": "string", "enum": ["html", "pdf", "png"]}),
                    "content".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["format".into(), "content".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "preview".into(),
            title: None,
            description: Some("Preview rendered output".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "render_id".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["render_id".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ]);

    // MCP server 2 also has "output"
    let conn2 = objectiveai::mcp::Connection::new_for_test(
        "test".into(),
        "https://logger.example.com/mcp".into(),
    );
    let mcp_tools2 = Arc::new(vec![
        objectiveai::mcp::tool::Tool {
            name: "output".into(),
            title: None,
            description: Some("Write to log output".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "level".into() => serde_json::json!({"type": "string", "enum": ["debug", "info", "warn", "error"]}),
                    "message".into() => serde_json::json!({"type": "string"}),
                }),
                required: Some(vec!["level".into(), "message".into()]),
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
        objectiveai::mcp::tool::Tool {
            name: "tail_logs".into(),
            title: None,
            description: Some("Tail recent log entries".into()),
            icons: None,
            input_schema: objectiveai::mcp::tool::ToolSchemaObject {
                r#type: objectiveai::mcp::tool::ToolSchemaType::Object,
                properties: Some(indexmap::indexmap! {
                    "n".into() => serde_json::json!({"type": "integer", "default": 50}),
                }),
                required: None,
                extra: indexmap::IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        },
    ]);

    // Invention tool also named "output"
    let invention_tools = vec![
        objectiveai::functions::inventions::InventionTool {
            name: "output",
            description: "Invention output formatter",
            parameters: indexmap::indexmap! {
                "type".into() => serde_json::json!("object"),
                "properties".into() => serde_json::json!({
                    "data": {"type": "object"},
                    "template": {"type": "string"}
                }),
                "required".into() => serde_json::json!(["data"]),
            },
            call: Arc::new(|_| Box::pin(async { Ok("formatted".into()) })),
        },
    ];

    let mcp_connections = vec![conn1, conn2];
    let mcp_tools = vec![mcp_tools1, mcp_tools2];

    let result = build_params(
        &agent,
        &params,
        &messages,
        None,
        &mcp_connections,
        &mcp_tools,
        Some(&invention_tools),
    );

    let expected = ChatCompletionCreateParams {
        messages: messages.clone(),
        provider: None,
        model: "anthropic/claude-sonnet-4".into(),
        frequency_penalty: None,
        logit_bias: None,
        max_completion_tokens: None,
        presence_penalty: None,
        stop: None,
        temperature: Some(0.5),
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
            // MCP1 renderer "output" gets (server_name(url)) suffix (same server name).
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "output (test(https://renderer.example.com/mcp))".into(),
                    description: Some("Render output to display".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::Value::Object(
                            vec![
                                ("format".into(), serde_json::json!({"type": "string", "enum": ["html", "pdf", "png"]})),
                                ("content".into(), serde_json::json!({"type": "string"})),
                            ].into_iter().collect(),
                        ),
                        "required".into() => serde_json::json!(["format", "content"]),
                    }),
                    strict: None,
                },
            },
            // MCP1 "preview" (unique, no suffix).
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "preview".into(),
                    description: Some("Preview rendered output".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::Value::Object(
                            vec![
                                ("render_id".into(), serde_json::json!({"type": "string"})),
                            ].into_iter().collect(),
                        ),
                        "required".into() => serde_json::json!(["render_id"]),
                    }),
                    strict: None,
                },
            },
            // MCP2 logger "output" gets (server_name(url)) suffix (same server name).
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "output (test(https://logger.example.com/mcp))".into(),
                    description: Some("Write to log output".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::Value::Object(
                            vec![
                                ("level".into(), serde_json::json!({"type": "string", "enum": ["debug", "info", "warn", "error"]})),
                                ("message".into(), serde_json::json!({"type": "string"})),
                            ].into_iter().collect(),
                        ),
                        "required".into() => serde_json::json!(["level", "message"]),
                    }),
                    strict: None,
                },
            },
            // MCP2 "tail_logs" (unique, no suffix).
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "tail_logs".into(),
                    description: Some("Tail recent log entries".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::Value::Object(
                            vec![
                                ("n".into(), serde_json::json!({"type": "integer", "default": 50})),
                            ].into_iter().collect(),
                        ),
                    }),
                    strict: None,
                },
            },
            // Invention "output" gets "(objectiveai-invention)" suffix.
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "output (objectiveai-invention)".into(),
                    description: Some("Invention output formatter".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::json!({
                            "data": {"type": "object"},
                            "template": {"type": "string"}
                        }),
                        "required".into() => serde_json::json!(["data"]),
                    }),
                    strict: None,
                },
            },
            // RF "output" keeps plain name (last, response format never gets suffix).
            super::Tool::Function {
                function: super::FunctionTool {
                    name: "output".into(),
                    description: Some("Structured output from RF".into()),
                    parameters: Some(indexmap::indexmap! {
                        "type".into() => serde_json::json!("object"),
                        "properties".into() => serde_json::json!({
                            "result": {"type": "string"}
                        }),
                    }),
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

#[test]
fn test_continuation_assistant_message_appended() {
    let agent = objectiveai::agent::openrouter::Agent {
        id: String::new(),
        base: objectiveai::agent::openrouter::AgentBase {
            model: "test-model".to_string(),
            ..Default::default()
        },
    };

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages = vec![objectiveai::agent::completions::message::Message::User(
        objectiveai::agent::completions::message::UserMessage {
            content: objectiveai::agent::completions::message::RichContent::Text(
                "Hello".to_string(),
            ),
            name: None,
        },
    )];

    let continuation = vec![
        crate::agent::completions::ContinuationItem::State(
            objectiveai::agent::completions::message::AssistantMessage {
                content: Some(objectiveai::agent::completions::message::RichContent::Text(
                    "Hi there!".to_string(),
                )),
                name: None,
                refusal: None,
                tool_calls: None,
                reasoning: None,
            },
        ),
    ];

    let mcp_connections: Vec<std::sync::Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<std::sync::Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        Some(&continuation),
        &mcp_connections,
        &mcp_tools,
        None,
    );

    let expected = ChatCompletionCreateParams {
        messages: vec![
            objectiveai::agent::completions::message::Message::User(
                objectiveai::agent::completions::message::UserMessage {
                    content: objectiveai::agent::completions::message::RichContent::Text(
                        "Hello".to_string(),
                    ),
                    name: None,
                },
            ),
            objectiveai::agent::completions::message::Message::Assistant(
                objectiveai::agent::completions::message::AssistantMessage {
                    content: Some(
                        objectiveai::agent::completions::message::RichContent::Text(
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

#[test]
fn test_continuation_mixed_items() {
    let agent = objectiveai::agent::openrouter::Agent {
        id: String::new(),
        base: objectiveai::agent::openrouter::AgentBase {
            model: "test-model".to_string(),
            ..Default::default()
        },
    };

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let messages = vec![objectiveai::agent::completions::message::Message::User(
        objectiveai::agent::completions::message::UserMessage {
            content: objectiveai::agent::completions::message::RichContent::Text(
                "What is the weather?".to_string(),
            ),
            name: None,
        },
    )];

    let continuation = vec![
        // Assistant made a tool call
        crate::agent::completions::ContinuationItem::State(
            objectiveai::agent::completions::message::AssistantMessage {
                content: None,
                name: None,
                refusal: None,
                tool_calls: Some(vec![
                    objectiveai::agent::completions::message::AssistantToolCall::Function {
                        id: "call_abc".to_string(),
                        function:
                            objectiveai::agent::completions::message::AssistantToolCallFunction {
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
            objectiveai::agent::completions::message::ToolMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Sunny, 72F".to_string(),
                ),
                tool_call_id: "call_abc".to_string(),
            },
        ),
        // User follow-up
        crate::agent::completions::ContinuationItem::UserMessage(
            objectiveai::agent::completions::message::UserMessage {
                content: objectiveai::agent::completions::message::RichContent::Text(
                    "Thanks! What about tomorrow?".to_string(),
                ),
                name: None,
            },
        ),
    ];

    let mcp_connections: Vec<std::sync::Arc<objectiveai::mcp::Connection>> = vec![];
    let mcp_tools: Vec<std::sync::Arc<Vec<objectiveai::mcp::tool::Tool>>> = vec![];

    let result = build_params(
        &agent,
        &params,
        &messages,
        Some(&continuation),
        &mcp_connections,
        &mcp_tools,
        None,
    );

    let expected = ChatCompletionCreateParams {
        messages: vec![
            objectiveai::agent::completions::message::Message::User(
                objectiveai::agent::completions::message::UserMessage {
                    content: objectiveai::agent::completions::message::RichContent::Text(
                        "What is the weather?".to_string(),
                    ),
                    name: None,
                },
            ),
            objectiveai::agent::completions::message::Message::Assistant(
                objectiveai::agent::completions::message::AssistantMessage {
                    content: None,
                    name: None,
                    refusal: None,
                    tool_calls: Some(vec![
                        objectiveai::agent::completions::message::AssistantToolCall::Function {
                            id: "call_abc".to_string(),
                            function:
                                objectiveai::agent::completions::message::AssistantToolCallFunction {
                                    name: "get_weather".to_string(),
                                    arguments: "{\"city\":\"NYC\"}".to_string(),
                                },
                        },
                    ]),
                    reasoning: None,
                },
            ),
            objectiveai::agent::completions::message::Message::Tool(
                objectiveai::agent::completions::message::ToolMessage {
                    content: objectiveai::agent::completions::message::RichContent::Text(
                        "Sunny, 72F".to_string(),
                    ),
                    tool_call_id: "call_abc".to_string(),
                },
            ),
            objectiveai::agent::completions::message::Message::User(
                objectiveai::agent::completions::message::UserMessage {
                    content: objectiveai::agent::completions::message::RichContent::Text(
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

#[test]
fn test_tools_disabled_sets_tool_choice_none() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    // Use an optional ToolCall response format so tools get resolved but
    // it's not required (required would be rejected earlier in the client).
    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: Some(
            objectiveai::agent::completions::request::ResponseFormatParam::Single(
                objectiveai::agent::completions::request::ResponseFormat::ToolCall {
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
        &agent, &params, &[], None, &[], &[], None, false,
    );

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

#[test]
fn test_tools_disabled_no_tools_no_tool_choice() {
    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![],
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
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
        &agent, &params, &[], None, &[], &[], None, false,
    );

    // No tools means no tool_choice at all.
    assert_eq!(result.tool_choice, None);
    assert!(result.tools.is_none());
}

#[test]
fn test_request_continuation_messages_come_first() {
    use objectiveai::agent::completions::message::*;

    let agent = objectiveai::agent::openrouter::Agent::try_from(
        objectiveai::agent::openrouter::AgentBase {
            model: "test-model".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Current turn".into()),
        name: None,
    })];

    let params = objectiveai::agent::completions::request::AgentCompletionCreateParams {
        messages: messages.clone(),
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase::default()),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: None,
        stream: None,
        continuation: None,
    };

    let request_continuation = objectiveai::agent::openrouter::Continuation {
        upstream: objectiveai::agent::openrouter::Upstream::default(),
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
