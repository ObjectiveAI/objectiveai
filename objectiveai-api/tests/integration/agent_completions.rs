//! Integration tests for the `/agent/completions` endpoint of the
//! spawned `objectiveai-api` server. Each test POSTs an
//! `AgentCompletionCreateParams` body, streams the SSE response, and
//! snapshots the aggregated `AgentCompletion`.

#![allow(clippy::too_many_arguments)]

use futures::StreamExt;
use objectiveai_sdk::agent::completions::message::{
    AssistantMessage, AssistantToolCall, AssistantToolCallFunction, DeveloperMessage, Message,
    RichContent, SimpleContent, UserMessage,
};
use objectiveai_sdk::agent::completions::request::{
    AgentCompletionCreateParams, ResponseFormat, ResponseFormatParam,
};
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
use objectiveai_sdk::agent::completions::response::unary::{AgentCompletion, Message as UnaryMessage};
use objectiveai_sdk::agent::mock::AgentBase as MockAgentBase;

use crate::common;

// ---------------------------------------------------------------------------
// Snapshot helpers
// ---------------------------------------------------------------------------

fn check_created_and_upstream(
    expected_created: &std::cell::Cell<Option<u64>>,
    expected_upstream: &std::cell::Cell<Option<objectiveai_sdk::agent::Upstream>>,
    i: usize,
    chunk: &AgentCompletionChunk,
) {
    match expected_created.get() {
        None => expected_created.set(Some(chunk.created)),
        Some(exp) => assert_eq!(
            chunk.created, exp,
            "chunk {i} has created {}, expected {exp}",
            chunk.created
        ),
    }
    match expected_upstream.get() {
        None => expected_upstream.set(Some(chunk.upstream)),
        Some(exp) => assert_eq!(
            chunk.upstream, exp,
            "chunk {i} has upstream {:?}, expected {:?}",
            chunk.upstream, exp
        ),
    }
}

async fn run_and_check(
    stream: impl futures::Stream<Item = AgentCompletionChunk> + Unpin,
) -> AgentCompletion {
    let expected_created = std::cell::Cell::new(None);
    let expected_upstream: std::cell::Cell<Option<objectiveai_sdk::agent::Upstream>> =
        std::cell::Cell::new(None);
    let agg = common::stream_harness::consume_stream(
        stream,
        |agg, c| agg.push(c),
        |i, chunk| {
            check_created_and_upstream(&expected_created, &expected_upstream, i, chunk);
            assert!(chunk.messages.len() <= 1, "chunk {i} has {} messages, expected at most 1", chunk.messages.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
            assert!(chunk.continuation.is_none(), "chunk {i} (non-final) has continuation, expected None");
        },
        |i, chunk| {
            check_created_and_upstream(&expected_created, &expected_upstream, i, chunk);
            assert!(chunk.messages.len() <= 1, "chunk {i} has {} messages, expected at most 1", chunk.messages.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
            assert!(chunk.continuation.is_none(), "chunk {i} (non-final) has continuation, expected None");
        },
        |i, chunk| {
            check_created_and_upstream(&expected_created, &expected_upstream, i, chunk);
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
            assert!(chunk.continuation.is_some(), "final chunk {i} has no continuation, expected Some");
        },
    )
    .await;
    AgentCompletion::from(agg)
}

fn normalize(mut c: AgentCompletion) -> AgentCompletion {
    c.normalize_for_tests();
    c
}

fn assert_snapshot(json: &str, path: &str, expected: &str) {
    common::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_AGENT_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

/// POST `params` to `/agent/completions` (streaming) on the spawned api
/// server and return the resulting `Stream<AgentCompletionChunk>`.
async fn post_streaming(
    params: AgentCompletionCreateParams,
) -> impl futures::Stream<Item = AgentCompletionChunk> + Unpin {
    let http = common::server::client();
    let stream = http
        .send_streaming::<AgentCompletionChunk, _, _>(
            reqwest::Method::POST,
            "/agent/completions",
            Some(params),
        )
        .await
        .expect("send_streaming should succeed");
    Box::pin(stream.map(|item| match item {
        Ok(chunk) => chunk,
        Err(e) => panic!("chunk deserialize / stream error: {e:?}"),
    }))
}

/// POST `params` and consume the stream until the first error
/// surfaces — either as a non-2xx HTTP status (BadStatus item) or
/// as an in-stream error chunk. Use for tests that assert the
/// request fails. Panics if the stream ends without an error.
async fn post_expect_error(params: AgentCompletionCreateParams) {
    let http = common::server::client();
    let mut stream = match http
        .send_streaming::<AgentCompletionChunk, _, _>(
            reqwest::Method::POST,
            "/agent/completions",
            Some(params),
        )
        .await
    {
        Ok(s) => Box::pin(s),
        Err(_) => return,
    };
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) if chunk.error.is_some() => return,
            Ok(_) => continue,
            Err(_) => return,
        }
    }
    panic!("expected an error, but stream ended without one");
}

/// Build a default mock agent (no error, no logprobs).
fn mock_agent(
    base: MockAgentBase,
    fallbacks: Option<Vec<objectiveai_sdk::agent::InlineAgentBase>>,
) -> objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional {
    objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
        objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
            inner: objectiveai_sdk::agent::InlineAgentBase::Mock(base),
            fallbacks,
        },
    )
}

fn default_mock() -> objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional {
    mock_agent(MockAgentBase::default(), None)
}

fn params_with(
    seed: i64,
    messages: Vec<Message>,
    response_format: Option<ResponseFormatParam>,
    agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
) -> AgentCompletionCreateParams {
    AgentCompletionCreateParams {
        messages,
        agent,
        provider: None,
        response_format,
        seed: Some(seed),
        stream: Some(true),
        continuation: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Default mock agent, no error.
#[tokio::test]
async fn test_basic_mock_agent_seed_42() {
    let stream = post_streaming(params_with(42, vec![], None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_basic_mock_agent_seed_42.json"),
        include_str!("../../assets/agent/completions/client_tests/test_basic_mock_agent_seed_42.json"),
    );
}

/// Default mock agent with seed 123.
#[tokio::test]
async fn test_basic_mock_agent_seed_123() {
    let stream = post_streaming(params_with(123, vec![], None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_basic_mock_agent_seed_123.json"),
        include_str!("../../assets/agent/completions/client_tests/test_basic_mock_agent_seed_123.json"),
    );
}

/// Same seed produces identical streams.
#[tokio::test]
async fn test_deterministic_with_same_seed() {
    let stream_a = post_streaming(params_with(77, vec![], None, default_mock())).await;
    let completion_a = normalize(run_and_check(stream_a).await);

    let stream_b = post_streaming(params_with(77, vec![], None, default_mock())).await;
    let completion_b = normalize(run_and_check(stream_b).await);

    assert_eq!(completion_a, completion_b);

    let json_a = serde_json::to_string_pretty(&completion_a).unwrap();
    assert_snapshot(
        &json_a,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_deterministic_with_same_seed.json"),
        include_str!("../../assets/agent/completions/client_tests/test_deterministic_with_same_seed.json"),
    );
}

/// Different seeds produce different streams.
#[tokio::test]
async fn test_different_seeds_differ() {
    let stream_a = post_streaming(params_with(1, vec![], None, default_mock())).await;
    let completion_a = normalize(run_and_check(stream_a).await);

    let stream_b = post_streaming(params_with(2, vec![], None, default_mock())).await;
    let completion_b = normalize(run_and_check(stream_b).await);

    assert_ne!(completion_a, completion_b);

    let json_a = serde_json::to_string_pretty(&completion_a).unwrap();
    assert_snapshot(
        &json_a,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_different_seeds_differ_a.json"),
        include_str!("../../assets/agent/completions/client_tests/test_different_seeds_differ_a.json"),
    );
    let json_b = serde_json::to_string_pretty(&completion_b).unwrap();
    assert_snapshot(
        &json_b,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_different_seeds_differ_b.json"),
        include_str!("../../assets/agent/completions/client_tests/test_different_seeds_differ_b.json"),
    );
}

/// Mock agent with error=true should fail (server emits an error chunk).
#[tokio::test]
async fn test_mock_agent_with_error() {
    let agent = mock_agent(
        MockAgentBase { error: Some(true), ..Default::default() },
        None,
    );
    post_expect_error(params_with(42, vec![], None, agent)).await;
}

/// Messages: single user message.
#[tokio::test]
async fn test_with_single_user_message() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Hello, world!".into()),
        name: None,
    })];
    let stream = post_streaming(params_with(42, messages, None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_with_single_user_message.json"),
        include_str!("../../assets/agent/completions/client_tests/test_with_single_user_message.json"),
    );
}

/// Messages: developer + user messages.
#[tokio::test]
async fn test_with_developer_and_user_messages() {
    let messages = vec![
        Message::Developer(DeveloperMessage {
            content: SimpleContent::Text("You are a helpful assistant.".into()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("What is 2+2?".into()),
            name: None,
        }),
    ];
    let stream = post_streaming(params_with(99, messages, None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_with_developer_and_user_messages.json"),
        include_str!("../../assets/agent/completions/client_tests/test_with_developer_and_user_messages.json"),
    );
}

/// Response format: JsonObject.
#[tokio::test]
async fn test_json_object_response_format() {
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::JsonObject));
    let stream = post_streaming(params_with(42, vec![], rf, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_json_object_response_format.json"),
        include_str!("../../assets/agent/completions/client_tests/test_json_object_response_format.json"),
    );
}

/// Response format: JsonSchema with object schema.
#[tokio::test]
async fn test_json_schema_response_format() {
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::JsonSchema {
        schema: indexmap::indexmap! {
            "type".into() => serde_json::json!("object"),
            "properties".into() => serde_json::json!({
                "name": {"type": "string"},
                "age": {"type": "integer"},
            }),
        },
    }));
    let stream = post_streaming(params_with(42, vec![], rf, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_json_schema_response_format.json"),
        include_str!("../../assets/agent/completions/client_tests/test_json_schema_response_format.json"),
    );
}

/// Response format: Text.
#[tokio::test]
async fn test_text_response_format() {
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::Text));
    let stream = post_streaming(params_with(77, vec![], rf, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_text_response_format.json"),
        include_str!("../../assets/agent/completions/client_tests/test_text_response_format.json"),
    );
}

/// Response format: Grammar should be rejected by mock client.
#[tokio::test]
async fn test_grammar_response_format_rejected() {
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::Grammar {
        grammar: "root ::= 'hello'".into(),
    }));
    post_expect_error(params_with(42, vec![], rf, default_mock())).await;
}

/// Response format: Python should be rejected by mock client.
#[tokio::test]
async fn test_python_response_format_rejected() {
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::Python));
    post_expect_error(params_with(42, vec![], rf, default_mock())).await;
}

/// Response format: ToolCall with required=true.
#[tokio::test]
async fn test_required_tool_call_response_format() {
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::ToolCall {
        name: "submit".into(),
        description: "Submit output".into(),
        schema: indexmap::indexmap! {
            "type".into() => serde_json::json!("object"),
            "properties".into() => serde_json::json!({
                "answer": {"type": "string"},
            }),
        },
        required: Some(true),
    }));
    let stream = post_streaming(params_with(42, vec![], rf, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_required_tool_call_response_format.json"),
        include_str!("../../assets/agent/completions/client_tests/test_required_tool_call_response_format.json"),
    );
}

/// Response format: ToolCall with required=None (optional).
#[tokio::test]
async fn test_optional_tool_call_response_format() {
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::ToolCall {
        name: "submit_answer".into(),
        description: "Submit the final answer".into(),
        schema: indexmap::indexmap! {
            "type".into() => serde_json::json!("object"),
            "properties".into() => serde_json::json!({
                "answer": {"type": "string"},
                "confidence": {"type": "number"},
            }),
        },
        required: None,
    }));
    let stream = post_streaming(params_with(200, vec![], rf, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_optional_tool_call_response_format.json"),
        include_str!("../../assets/agent/completions/client_tests/test_optional_tool_call_response_format.json"),
    );
}

/// Multiple user messages in a conversation.
#[tokio::test]
async fn test_multiple_user_messages() {
    let messages = vec![
        Message::User(UserMessage {
            content: RichContent::Text("First message".into()),
            name: None,
        }),
        Message::User(UserMessage {
            content: RichContent::Text("Second message".into()),
            name: Some("alice".into()),
        }),
    ];
    let stream = post_streaming(params_with(55, messages, None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_multiple_user_messages.json"),
        include_str!("../../assets/agent/completions/client_tests/test_multiple_user_messages.json"),
    );
}

/// Mock agent with error=Some(false) should succeed (normalized to None).
#[tokio::test]
async fn test_mock_agent_error_false_succeeds() {
    let agent = mock_agent(
        MockAgentBase { error: Some(false), ..Default::default() },
        None,
    );
    let stream = post_streaming(params_with(42, vec![], None, agent)).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_mock_agent_error_false_succeeds.json"),
        include_str!("../../assets/agent/completions/client_tests/test_mock_agent_error_false_succeeds.json"),
    );
}

/// Final stream item is always a Continuation::Mock.
#[tokio::test]
async fn test_final_item_is_mock_continuation() {
    let stream = post_streaming(params_with(42, vec![], None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_final_item_is_mock_continuation.json"),
        include_str!("../../assets/agent/completions/client_tests/test_final_item_is_mock_continuation.json"),
    );
}

/// PerAgent response format targeting the mock agent's ID.
#[tokio::test]
async fn test_per_agent_response_format() {
    let mock_base = MockAgentBase::default();
    let agent_id = mock_base.id();

    let mut per_agent = indexmap::IndexMap::new();
    per_agent.insert(agent_id, ResponseFormat::JsonObject);

    let rf = Some(ResponseFormatParam::PerAgent(per_agent));
    let agent = mock_agent(mock_base, None);
    let stream = post_streaming(params_with(42, vec![], rf, agent)).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_per_agent_response_format.json"),
        include_str!("../../assets/agent/completions/client_tests/test_per_agent_response_format.json"),
    );
}

/// PerAgent response format with unknown agent ID (should fall back to no format).
#[tokio::test]
async fn test_per_agent_response_format_unknown_id() {
    let mut per_agent = indexmap::IndexMap::new();
    per_agent.insert("nonexistent_agent_id_12345".into(), ResponseFormat::JsonObject);
    let rf = Some(ResponseFormatParam::PerAgent(per_agent));
    let stream = post_streaming(params_with(42, vec![], rf, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_per_agent_response_format_unknown_id.json"),
        include_str!("../../assets/agent/completions/client_tests/test_per_agent_response_format_unknown_id.json"),
    );
}

/// JsonSchema with nested object schema.
#[tokio::test]
async fn test_json_schema_nested_object() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Generate a person".into()),
        name: None,
    })];
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::JsonSchema {
        schema: indexmap::indexmap! {
            "type".into() => serde_json::json!("object"),
            "properties".into() => serde_json::json!({
                "person": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "address": {
                            "type": "object",
                            "properties": {
                                "street": {"type": "string"},
                                "city": {"type": "string"},
                            }
                        }
                    }
                }
            }),
        },
    }));
    let stream = post_streaming(params_with(99, messages, rf, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_json_schema_nested_object.json"),
        include_str!("../../assets/agent/completions/client_tests/test_json_schema_nested_object.json"),
    );
}

/// Fallback agents: primary errors, fallback succeeds.
#[tokio::test]
async fn test_fallback_agent_on_error() {
    let agent = mock_agent(
        MockAgentBase { error: Some(true), ..Default::default() },
        Some(vec![objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase::default())]),
    );
    let stream = post_streaming(params_with(42, vec![], None, agent)).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_fallback_agent_on_error.json"),
        include_str!("../../assets/agent/completions/client_tests/test_fallback_agent_on_error.json"),
    );
}

/// Both primary and fallback agents error — should fail.
#[tokio::test]
async fn test_all_agents_error() {
    let agent = mock_agent(
        MockAgentBase { error: Some(true), ..Default::default() },
        Some(vec![objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase {
            error: Some(true),
            ..Default::default()
        })]),
    );
    post_expect_error(params_with(42, vec![], None, agent)).await;
}

/// Multiple fallback agents — first two error, third succeeds.
#[tokio::test]
async fn test_multiple_fallback_agents() {
    let agent = mock_agent(
        MockAgentBase { error: Some(true), ..Default::default() },
        Some(vec![
            objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase {
                error: Some(true),
                ..Default::default()
            }),
            objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase::default()),
        ]),
    );
    let stream = post_streaming(params_with(42, vec![], None, agent)).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_multiple_fallback_agents.json"),
        include_str!("../../assets/agent/completions/client_tests/test_multiple_fallback_agents.json"),
    );
}

/// Build a base64-encoded `Continuation::Mock` for use as the `continuation` field.
fn encoded_mock_continuation(messages: Vec<Message>) -> String {
    let cont = objectiveai_sdk::agent::Continuation::Mock(objectiveai_sdk::agent::mock::Continuation {
        upstream: objectiveai_sdk::agent::mock::Upstream::Mock,
        agent_id: String::new(),
        messages,
        mcp_sessions: indexmap::IndexMap::new(),
        ws_session_id: None,
    });
    cont.to_string()
}

/// With continuation from a previous Mock run.
#[tokio::test]
async fn test_with_mock_continuation() {
    let cont = encoded_mock_continuation(vec![Message::Assistant(AssistantMessage {
        content: None, name: None, refusal: None, tool_calls: None, reasoning: None,
    })]);

    let mut params = params_with(42, vec![], None, default_mock());
    params.continuation = Some(cont);

    let stream = post_streaming(params).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_with_mock_continuation.json"),
        include_str!("../../assets/agent/completions/client_tests/test_with_mock_continuation.json"),
    );
}

/// Stream produces chunks before the final state.
#[tokio::test]
async fn test_stream_yields_chunks_before_state() {
    let stream = post_streaming(params_with(42, vec![], None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_stream_yields_chunks_before_state.json"),
        include_str!("../../assets/agent/completions/client_tests/test_stream_yields_chunks_before_state.json"),
    );
}

/// Large seed value.
#[tokio::test]
async fn test_large_seed_value() {
    let stream = post_streaming(params_with(u64::MAX as i64, vec![], None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_large_seed_value.json"),
        include_str!("../../assets/agent/completions/client_tests/test_large_seed_value.json"),
    );
}

/// Seed 0.
#[tokio::test]
async fn test_seed_zero() {
    let stream = post_streaming(params_with(0, vec![], None, default_mock())).await;
    let completion = normalize(run_and_check(stream).await);
    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_seed_zero.json"),
        include_str!("../../assets/agent/completions/client_tests/test_seed_zero.json"),
    );
}

// ---------------------------------------------------------------------------
// Logprobs helpers
// ---------------------------------------------------------------------------

/// Asserts that every assistant message with content also has logprobs whose
/// tokens concatenate to reconstruct the content text.
fn assert_completion_logprobs(completion: &AgentCompletion) {
    for (i, msg) in completion.messages.iter().enumerate() {
        let asst = match msg {
            UnaryMessage::Assistant(a) => a,
            _ => continue,
        };
        let content = match &asst.content {
            Some(RichContent::Text(t)) => t.as_str(),
            _ => continue,
        };
        let logprobs = match &asst.logprobs {
            Some(lps) => lps,
            None => panic!("message {i}: assistant has content but no logprobs"),
        };
        let content_lps = match &logprobs.content {
            Some(lps) => lps,
            None => panic!("message {i}: logprobs present but content logprobs missing"),
        };
        let reconstructed: String = content_lps.iter().map(|lp| lp.token.as_str()).collect();
        assert_eq!(
            reconstructed, content,
            "message {i}: logprob tokens don't reconstruct content",
        );
    }
}

// ---------------------------------------------------------------------------
// Logprobs tests
// ---------------------------------------------------------------------------

/// Basic logprobs with plain text, no tools, no response format.
#[tokio::test]
async fn test_logprobs_basic_seed_42() {
    let messages = vec![Message::User(UserMessage {
        content: RichContent::Text("Tell me something".into()),
        name: None,
    })];
    let agent = mock_agent(
        MockAgentBase { top_logprobs: Some(5), ..Default::default() },
        None,
    );
    let stream = post_streaming(params_with(42, messages, None, agent)).await;
    let completion = normalize(run_and_check(stream).await);
    assert_completion_logprobs(&completion);

    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_logprobs_basic_seed_42.json"),
        include_str!("../../assets/agent/completions/client_tests/test_logprobs_basic_seed_42.json"),
    );
}

/// Logprobs with nested json_schema response format.
#[tokio::test]
async fn test_logprobs_json_schema_nested() {
    let agent = mock_agent(
        MockAgentBase { top_logprobs: Some(10), ..Default::default() },
        None,
    );
    let rf = Some(ResponseFormatParam::Single(ResponseFormat::JsonSchema {
        schema: indexmap::indexmap! {
            "type".into() => serde_json::json!("object"),
            "properties".into() => serde_json::json!({
                "result": {
                    "type": "object",
                    "properties": {
                        "label": {"type": "string"},
                        "values": {
                            "type": "array",
                            "items": {"type": "number"},
                        },
                    },
                },
            }),
        },
    }));
    let stream = post_streaming(params_with(77, vec![], rf, agent)).await;
    let completion = normalize(run_and_check(stream).await);
    assert_completion_logprobs(&completion);

    for msg in &completion.messages {
        if let UnaryMessage::Assistant(asst) = msg {
            if let Some(RichContent::Text(t)) = &asst.content {
                serde_json::from_str::<serde_json::Value>(t)
                    .expect("json_schema content should be valid JSON");
            }
        }
    }

    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_logprobs_json_schema_nested.json"),
        include_str!("../../assets/agent/completions/client_tests/test_logprobs_json_schema_nested.json"),
    );
}

/// Logprobs survive through the continuation flow.
#[tokio::test]
async fn test_logprobs_with_continuation() {
    let agent = mock_agent(
        MockAgentBase { top_logprobs: Some(7), ..Default::default() },
        None,
    );
    let cont = encoded_mock_continuation(vec![Message::Assistant(AssistantMessage {
        content: None,
        name: None,
        refusal: None,
        reasoning: None,
        tool_calls: Some(vec![
            AssistantToolCall::Function {
                id: "1".into(),
                function: AssistantToolCallFunction { name: "a".into(), arguments: String::new() },
            },
            AssistantToolCall::Function {
                id: "2".into(),
                function: AssistantToolCallFunction { name: "b".into(), arguments: String::new() },
            },
            AssistantToolCall::Function {
                id: "3".into(),
                function: AssistantToolCallFunction { name: "c".into(), arguments: String::new() },
            },
        ]),
    })]);

    let mut params = params_with(42, vec![], None, agent);
    params.continuation = Some(cont);

    let stream = post_streaming(params).await;
    let completion = normalize(run_and_check(stream).await);
    assert_completion_logprobs(&completion);

    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_logprobs_with_continuation.json"),
        include_str!("../../assets/agent/completions/client_tests/test_logprobs_with_continuation.json"),
    );
}

/// Primary agent errors, fallback agent has logprobs enabled.
#[tokio::test]
async fn test_logprobs_fallback_agent() {
    let agent = mock_agent(
        MockAgentBase { error: Some(true), ..Default::default() },
        Some(vec![
            objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase {
                top_logprobs: Some(12),
                ..Default::default()
            }),
        ]),
    );
    let stream = post_streaming(params_with(55, vec![], None, agent)).await;
    let completion = normalize(run_and_check(stream).await);
    assert_completion_logprobs(&completion);

    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_logprobs_fallback_agent.json"),
        include_str!("../../assets/agent/completions/client_tests/test_logprobs_fallback_agent.json"),
    );
}

/// Logprobs with PerAgent response format targeting mock agent's ID.
#[tokio::test]
async fn test_logprobs_per_agent_json_object() {
    let mock_base = MockAgentBase { top_logprobs: Some(4), ..Default::default() };
    let agent_id = mock_base.id();

    let mut per_agent = indexmap::IndexMap::new();
    per_agent.insert(agent_id, ResponseFormat::JsonObject);

    let messages = vec![Message::Developer(DeveloperMessage {
        content: SimpleContent::Text("Respond with JSON".into()),
        name: None,
    })];
    let rf = Some(ResponseFormatParam::PerAgent(per_agent));
    let agent = mock_agent(mock_base, None);
    let stream = post_streaming(params_with(33, messages, rf, agent)).await;
    let completion = normalize(run_and_check(stream).await);
    assert_completion_logprobs(&completion);

    for msg in &completion.messages {
        if let UnaryMessage::Assistant(asst) = msg {
            if let Some(RichContent::Text(t)) = &asst.content {
                serde_json::from_str::<serde_json::Value>(t)
                    .expect("per-agent json_object content should be valid JSON");
            }
        }
    }

    let json = serde_json::to_string_pretty(&completion).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/agent/completions/client_tests/test_logprobs_per_agent_json_object.json"),
        include_str!("../../assets/agent/completions/client_tests/test_logprobs_per_agent_json_object.json"),
    );
}
