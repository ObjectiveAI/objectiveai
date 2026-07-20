//! Integration tests for the `/vector/completions` endpoint of the
//! spawned `objectiveai-api` server.

#![allow(clippy::too_many_arguments)]

use futures::StreamExt;
use rust_decimal::Decimal;

use objectiveai_sdk::agent::completions::message::{
    File as MessageFile, ImageUrl, Message, RichContent, RichContentPart, UserMessage, VideoUrl,
};
use objectiveai_sdk::agent::mock::{
    AgentBase as MockAgentBase, OutputMode as MockOutputMode, Upstream as MockUpstream,
};
use objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams;
use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;
use objectiveai_sdk::vector::completions::response::unary::VectorCompletion;

use crate::common;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Helper to construct a mock agent for swarms.
fn mock_agent(
    output_mode: MockOutputMode,
    count: u64,
    top_logprobs: Option<u64>,
    error: Option<bool>,
    fallbacks: Option<Vec<objectiveai_sdk::agent::InlineAgentBase>>,
) -> objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
    objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
        count,
        inner: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase {
                    upstream: MockUpstream::Mock,
                    output_mode,
                    top_logprobs,
                    error,
                    error_probability: None,
                    mcp_servers: None,
                    laboratories: None,
                    objectiveai_mcp: None,
                    plugins: Vec::new(),
                    calls: None,
                }),
                fallbacks,
            },
        ),
    }
}

fn check_created(expected: &std::cell::Cell<Option<u64>>, i: usize, created: u64) {
    match expected.get() {
        None => expected.set(Some(created)),
        Some(exp) => assert_eq!(created, exp, "chunk {i} has created {created}, expected {exp}"),
    }
}

async fn run_and_check(
    stream: impl futures::Stream<Item = VectorCompletionChunk> + Unpin,
) -> VectorCompletion {
    let expected_created = std::cell::Cell::new(None);
    let agg = common::stream_harness::consume_stream(
        stream,
        |agg, c| agg.push(c),
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.completions.len() <= 1, "chunk {i} has {} completions, expected at most 1", chunk.completions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.completions.len() <= 1, "chunk {i} has {} completions, expected at most 1", chunk.completions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
        },
    )
    .await;
    VectorCompletion::from(agg)
}

fn normalize(mut vc: VectorCompletion) -> VectorCompletion {
    vc.normalize_for_tests();
    vc
}

fn assert_snapshot(json: &str, path: &str, expected: &str) {
    common::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS",
    );
}

/// POST `params` to `/vector/completions` (streaming) on the spawned api
/// server and return the resulting `Stream<VectorCompletionChunk>`.
async fn post_streaming(
    params: VectorCompletionCreateParams,
) -> impl futures::Stream<Item = VectorCompletionChunk> + Unpin {
    let http = common::server::client();
    let stream = http
        .send_streaming::<VectorCompletionChunk, _, _>(
            reqwest::Method::POST,
            "/vector/completions",
            Some(params),
        )
        .await
        .expect("send_streaming should succeed");
    Box::pin(stream.map(|item| match item {
        Ok(chunk) => chunk,
        Err(e) => panic!("chunk deserialize / stream error: {e:?}"),
    }))
}

/// POST `params` and return the surfaced error message — either a non-2xx
/// HTTP body's `message` field, an inner agent-completion error chunk's
/// `message`, or a stream deserialization error. Panics if the stream
/// completes successfully without surfacing any error.
async fn post_expect_error_msg(params: VectorCompletionCreateParams) -> String {
    let http = common::server::client();
    let stream_result = http
        .send_streaming::<VectorCompletionChunk, _, _>(
            reqwest::Method::POST,
            "/vector/completions",
            Some(params),
        )
        .await;
    let mut stream = match stream_result {
        Ok(s) => Box::pin(s),
        Err(e) => return format!("{e:?}"),
    };
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                for inner in &chunk.completions {
                    if let Some(err) = &inner.inner.error {
                        return err.message.to_string();
                    }
                }
            }
            Err(e) => return format!("{e:?}"),
        }
    }
    panic!("expected an error, but stream ended without one");
}

fn user_text(text: &str) -> Vec<Message> {
    vec![Message::User(UserMessage {
        content: RichContent::Text(text.to_string()),
    })]
}

fn user_parts(parts: Vec<RichContentPart>) -> Vec<Message> {
    vec![Message::User(UserMessage {
        content: RichContent::Parts(parts),
    })]
}

fn swarm(
    agents: Vec<objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount>,
    weights: objectiveai_sdk::Weights,
) -> objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional {
    objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
        objectiveai_sdk::swarm::InlineSwarmBase {
            agents,
            weights: Some(weights),
        },
    )
}

fn equal_weights() -> objectiveai_sdk::Weights {
    objectiveai_sdk::Weights::Weights(vec![Decimal::ONE])
}

fn weights(values: Vec<Decimal>) -> objectiveai_sdk::Weights {
    objectiveai_sdk::Weights::Weights(values)
}

fn params(
    seed: i64,
    messages: Vec<Message>,
    swarm: objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional,
    responses: Vec<RichContent>,
) -> VectorCompletionCreateParams {
    VectorCompletionCreateParams {
        messages,
        provider: None,
        swarm,
        seed: Some(seed),
        stream: Some(true),
        responses,
        continuation: None,
    }
}

fn responses_text(rs: Vec<&str>) -> Vec<RichContent> {
    rs.into_iter().map(|s| RichContent::Text(s.to_string())).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Single mock agent, 2 responses, instruction mode, seed 42.
#[tokio::test]
async fn test_single_agent_2_responses_instruction_seed_42() {
    let request = params(
        42,
        user_text("Which is better?"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec!["Response A", "Response B"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/single_agent_2_responses_instruction_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/single_agent_2_responses_instruction_seed_42.json"),
    );
}

/// Single mock agent, 3 responses, instruction mode, seed 42.
#[tokio::test]
async fn test_single_agent_3_responses_instruction_seed_42() {
    let request = params(
        42,
        user_text("Which is best?"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec!["Alpha", "Beta", "Gamma"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/single_agent_3_responses_instruction_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/single_agent_3_responses_instruction_seed_42.json"),
    );
}

/// Two mock agents with equal weights, seed 42.
#[tokio::test]
async fn test_two_agents_equal_weights_seed_42() {
    let request = params(
        42,
        user_text("Pick one"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 2, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec!["Option 1", "Option 2"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 4);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/two_agents_equal_weights_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/two_agents_equal_weights_seed_42.json"),
    );
}

/// Two different mock agent definitions with unequal weights (0.8 / 0.2), seed 42.
#[tokio::test]
async fn test_two_agents_unequal_weights_seed_42() {
    let request = params(
        42,
        user_text("Pick one"),
        swarm(
            vec![
                mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                mock_agent(MockOutputMode::Instruction, 1, None, None, None),
            ],
            weights(vec![Decimal::new(8, 1), Decimal::new(2, 1)]),
        ),
        responses_text(vec!["Option 1", "Option 2"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 4);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/two_agents_unequal_weights_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/two_agents_unequal_weights_seed_42.json"),
    );
}

/// Three agents (via count=3), 4 responses, seed 99.
#[tokio::test]
async fn test_three_agents_4_responses_seed_99() {
    let request = params(
        99,
        user_text("Rank these"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 3, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec!["First", "Second", "Third", "Fourth"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/three_agents_4_responses_seed_99.json"),
        include_str!("../../assets/vector/completions/client_tests/three_agents_4_responses_seed_99.json"),
    );
}

/// Invert vote with single agent, seed 42.
#[tokio::test]
async fn test_invert_vote_seed_42() {
    let request = params(
        42,
        user_text("Which is worse?"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
            objectiveai_sdk::Weights::Entries(vec![objectiveai_sdk::WeightsEntry {
                weight: Decimal::ONE,
                invert: Some(true),
            }]),
        ),
        responses_text(vec!["Bad option", "Worse option"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/invert_vote_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/invert_vote_seed_42.json"),
    );
}

/// Same seed produces same result (deterministic).
#[tokio::test]
async fn test_deterministic_same_seed() {
    let make_request = || {
        params(
            42,
            user_text("Pick one"),
            swarm(
                vec![mock_agent(MockOutputMode::Instruction, 2, None, None, None)],
                equal_weights(),
            ),
            responses_text(vec!["A", "B", "C"]),
        )
    };

    let stream1 = post_streaming(make_request()).await;
    let result1 = normalize(run_and_check(stream1).await);
    let stream2 = post_streaming(make_request()).await;
    let result2 = normalize(run_and_check(stream2).await);

    let json1 = serde_json::to_string_pretty(&result1).unwrap();
    let json2 = serde_json::to_string_pretty(&result2).unwrap();
    assert_eq!(json1, json2, "same seed should produce identical results");
}

/// Different seeds produce different results.
#[tokio::test]
async fn test_different_seeds_differ() {
    let make_request = |seed: i64| {
        params(
            seed,
            user_text("Pick one"),
            swarm(
                vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
                equal_weights(),
            ),
            responses_text(vec!["A", "B"]),
        )
    };

    let stream1 = post_streaming(make_request(42)).await;
    let result1 = normalize(run_and_check(stream1).await);
    let stream2 = post_streaming(make_request(99)).await;
    let result2 = normalize(run_and_check(stream2).await);

    let json1 = serde_json::to_string_pretty(&result1).unwrap();
    let json2 = serde_json::to_string_pretty(&result2).unwrap();
    assert_ne!(json1, json2, "different seeds should produce different results");
}

/// Many responses (25) to test deep prefix tree, seed 42.
#[tokio::test]
async fn test_many_responses_deep_prefix_tree_seed_42() {
    let responses: Vec<RichContent> = (0..25)
        .map(|i| RichContent::Text(format!("Response {}", i)))
        .collect();
    let request = params(
        42,
        user_text("Pick the best"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
            equal_weights(),
        ),
        responses,
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/many_responses_deep_prefix_tree_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/many_responses_deep_prefix_tree_seed_42.json"),
    );
}

/// Single agent with json_schema output mode, seed 77.
#[tokio::test]
async fn test_json_schema_single_agent_seed_77() {
    let request = params(
        77,
        user_text("Rate the following essays on clarity"),
        swarm(
            vec![mock_agent(MockOutputMode::JsonSchema, 1, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec![
            "Essay about climate change",
            "Essay about artificial intelligence",
            "Essay about space exploration",
        ]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 1);
    assert_eq!(result.votes.len(), 1);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/json_schema_single_agent_seed_77.json"),
        include_str!("../../assets/vector/completions/client_tests/json_schema_single_agent_seed_77.json"),
    );
}

/// Single agent with tool_call output mode, seed 55.
#[tokio::test]
async fn test_tool_call_single_agent_seed_55() {
    let request = params(
        55,
        user_text("Which logo design is most memorable?"),
        swarm(
            vec![mock_agent(MockOutputMode::ToolCall, 1, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec!["Minimalist wordmark", "Abstract geometric icon"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert!(!result.completions.is_empty(), "should have at least one completion");
    assert_eq!(result.votes.len(), 1);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/tool_call_single_agent_seed_55.json"),
        include_str!("../../assets/vector/completions/client_tests/tool_call_single_agent_seed_55.json"),
    );
}

/// Single error agent — completion should contain an error, no votes.
#[tokio::test]
async fn test_error_agent_skipped_seed_42() {
    let request = params(
        42,
        user_text("Evaluate these proposals"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 1, None, Some(true), None)],
            equal_weights(),
        ),
        responses_text(vec!["Proposal A", "Proposal B"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 1);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/error_agent_skipped_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/error_agent_skipped_seed_42.json"),
    );
}

/// Mixed output modes: instruction + json_schema + tool_call agents, seed 88.
#[tokio::test]
async fn test_mixed_output_modes_seed_88() {
    let request = params(
        88,
        user_text("Compare these vacation destinations"),
        swarm(
            vec![
                mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                mock_agent(MockOutputMode::JsonSchema, 1, None, None, None),
                mock_agent(MockOutputMode::ToolCall, 1, None, None, None),
            ],
            weights(vec![
                Decimal::new(4, 1),
                Decimal::new(3, 1),
                Decimal::new(3, 1),
            ]),
        ),
        responses_text(vec!["Kyoto, Japan", "Reykjavik, Iceland", "Patagonia, Argentina"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/mixed_output_modes_seed_88.json"),
        include_str!("../../assets/vector/completions/client_tests/mixed_output_modes_seed_88.json"),
    );
}

/// Image responses with instruction mode, seed 33.
#[tokio::test]
async fn test_image_responses_instruction_seed_33() {
    let img_response = |url: &str, label: &str| {
        RichContent::Parts(vec![
            RichContentPart::ImageUrl {
                image_url: ImageUrl { url: url.to_string(), detail: None },
            },
            RichContentPart::Text { text: label.to_string() },
        ])
    };

    let request = params(
        33,
        user_text("Which painting has the best composition?"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
            equal_weights(),
        ),
        vec![
            img_response("https://example.com/painting-a.jpg", "Sunset over mountains"),
            img_response("https://example.com/painting-b.jpg", "Abstract cubist portrait"),
            img_response("https://example.com/painting-c.jpg", "Watercolor garden scene"),
        ],
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/image_responses_instruction_seed_33.json"),
        include_str!("../../assets/vector/completions/client_tests/image_responses_instruction_seed_33.json"),
    );
}

/// Video and file responses with json_schema mode, seed 66.
#[tokio::test]
async fn test_video_and_file_responses_seed_66() {
    let messages = user_parts(vec![
        RichContentPart::Text { text: "Review these submissions and pick the best one".to_string() },
        RichContentPart::VideoUrl { video_url: VideoUrl { url: "https://example.com/demo-reel.mp4".to_string() } },
    ]);

    let responses = vec![
        RichContent::Parts(vec![
            RichContentPart::VideoUrl { video_url: VideoUrl { url: "https://example.com/submission-1.mp4".to_string() } },
            RichContentPart::Text { text: "30-second product demo".to_string() },
        ]),
        RichContent::Parts(vec![
            RichContentPart::File { file: MessageFile {
                file_data: None, file_id: None,
                filename: Some("business-plan.pdf".to_string()),
                file_url: Some("https://example.com/business-plan.pdf".to_string()),
            } },
            RichContentPart::Text { text: "Written business plan".to_string() },
        ]),
        RichContent::Parts(vec![
            RichContentPart::VideoUrl { video_url: VideoUrl { url: "https://example.com/submission-3.mp4".to_string() } },
            RichContentPart::File { file: MessageFile {
                file_data: None, file_id: None,
                filename: Some("appendix.pdf".to_string()),
                file_url: Some("https://example.com/appendix.pdf".to_string()),
            } },
        ]),
    ];

    let request = params(
        66,
        messages,
        swarm(
            vec![mock_agent(MockOutputMode::JsonSchema, 1, None, None, None)],
            equal_weights(),
        ),
        responses,
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 1);
    assert_eq!(result.votes.len(), 1);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/video_and_file_responses_seed_66.json"),
        include_str!("../../assets/vector/completions/client_tests/video_and_file_responses_seed_66.json"),
    );
}

/// Three distinct agent definitions (instruction, json_schema, tool_call), seed 11.
#[tokio::test]
async fn test_three_different_agents_seed_11() {
    let messages = user_parts(vec![
        RichContentPart::Text { text: "Which dish looks the most appetizing?".to_string() },
        RichContentPart::ImageUrl {
            image_url: ImageUrl { url: "https://example.com/menu-context.jpg".to_string(), detail: None },
        },
    ]);

    let request = params(
        11,
        messages,
        swarm(
            vec![
                mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                mock_agent(MockOutputMode::JsonSchema, 1, None, None, None),
                mock_agent(MockOutputMode::ToolCall, 1, None, None, None),
            ],
            weights(vec![Decimal::new(5, 1), Decimal::new(3, 1), Decimal::new(2, 1)]),
        ),
        responses_text(vec![
            "Truffle risotto",
            "Seared tuna tataki",
            "Wagyu beef carpaccio",
            "Lobster thermidor",
        ]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert!(result.completions.len() >= 3, "should have at least one completion per agent");
    assert_eq!(result.votes.len(), 2);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/three_different_agents_seed_11.json"),
        include_str!("../../assets/vector/completions/client_tests/three_different_agents_seed_11.json"),
    );
}

/// Json_schema mode with 8 responses, seed 22.
#[tokio::test]
async fn test_json_schema_many_responses_seed_22() {
    let request = params(
        22,
        user_text("Rank these programming languages by expressiveness"),
        swarm(
            vec![mock_agent(MockOutputMode::JsonSchema, 2, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec![
            "Rust", "Haskell", "Python", "Lisp",
            "APL", "Forth", "Prolog", "Smalltalk",
        ]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 2);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/json_schema_many_responses_seed_22.json"),
        include_str!("../../assets/vector/completions/client_tests/json_schema_many_responses_seed_22.json"),
    );
}

/// Two tool_call agents with image message, seed 44.
#[tokio::test]
async fn test_tool_call_two_agents_seed_44() {
    let messages = user_parts(vec![
        RichContentPart::Text { text: "Which UI mockup should we go with?".to_string() },
        RichContentPart::ImageUrl {
            image_url: ImageUrl { url: "https://example.com/current-design.png".to_string(), detail: None },
        },
    ]);
    let img_response = |url: &str, label: &str| {
        RichContent::Parts(vec![
            RichContentPart::ImageUrl {
                image_url: ImageUrl { url: url.to_string(), detail: None },
            },
            RichContentPart::Text { text: label.to_string() },
        ])
    };

    let request = params(
        44,
        messages,
        swarm(
            vec![
                mock_agent(MockOutputMode::ToolCall, 1, None, None, None),
                mock_agent(MockOutputMode::ToolCall, 1, None, None, None),
            ],
            weights(vec![Decimal::new(6, 1), Decimal::new(4, 1)]),
        ),
        vec![
            img_response("https://example.com/mockup-a.png", "Clean flat design"),
            img_response("https://example.com/mockup-b.png", "Skeuomorphic with gradients"),
        ],
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/tool_call_two_agents_seed_44.json"),
        include_str!("../../assets/vector/completions/client_tests/tool_call_two_agents_seed_44.json"),
    );
}

/// One error agent + two healthy agents (json_schema, instruction), seed 99.
#[tokio::test]
async fn test_error_and_healthy_agents_seed_99() {
    let messages = user_parts(vec![
        RichContentPart::Text { text: "Evaluate these architectural plans".to_string() },
        RichContentPart::File { file: MessageFile {
            file_data: None, file_id: None,
            filename: Some("site-survey.pdf".to_string()),
            file_url: Some("https://example.com/site-survey.pdf".to_string()),
        } },
    ]);

    let responses = vec![
        RichContent::Parts(vec![
            RichContentPart::File { file: MessageFile {
                file_data: None, file_id: None,
                filename: Some("plan-modern.pdf".to_string()),
                file_url: Some("https://example.com/plan-modern.pdf".to_string()),
            } },
            RichContentPart::Text { text: "Modern glass facade".to_string() },
        ]),
        RichContent::Parts(vec![
            RichContentPart::File { file: MessageFile {
                file_data: None, file_id: None,
                filename: Some("plan-traditional.pdf".to_string()),
                file_url: Some("https://example.com/plan-traditional.pdf".to_string()),
            } },
            RichContentPart::Text { text: "Traditional brick and stone".to_string() },
        ]),
        RichContent::Text("Brutalist concrete monolith".to_string()),
    ];

    let request = params(
        99,
        messages,
        swarm(
            vec![
                mock_agent(MockOutputMode::Instruction, 1, None, Some(true), None),
                mock_agent(MockOutputMode::JsonSchema, 1, None, None, None),
                mock_agent(MockOutputMode::Instruction, 1, None, None, None),
            ],
            weights(vec![Decimal::new(3, 1), Decimal::new(4, 1), Decimal::new(3, 1)]),
        ),
        responses,
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.completions.len(), 4);
    assert_eq!(result.votes.len(), 1);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/error_and_healthy_agents_seed_99.json"),
        include_str!("../../assets/vector/completions/client_tests/error_and_healthy_agents_seed_99.json"),
    );
}

/// Only the final chunk should carry usage; all earlier chunks should have usage: None.
#[tokio::test]
async fn test_only_final_chunk_has_usage() {
    let request = params(
        42,
        user_text("Pick one"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 2, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec!["A", "B"]),
    );
    let stream = post_streaming(request).await;
    let _ = run_and_check(stream).await;
}

// ---------------------------------------------------------------------------
// Error tests (validation failures)
// ---------------------------------------------------------------------------

/// Zero responses → ExpectedTwoOrMoreRequestVectorResponses(0).
#[tokio::test]
async fn test_error_zero_responses() {
    let request = params(
        42,
        user_text("Pick one"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
            equal_weights(),
        ),
        vec![],
    );
    let msg = post_expect_error_msg(request).await;
    assert!(
        msg.contains("expected two or more") && msg.contains("got 0"),
        "unexpected error: {msg}"
    );
}

/// One response → ExpectedTwoOrMoreRequestVectorResponses(1).
#[tokio::test]
async fn test_error_one_response() {
    let request = params(
        42,
        user_text("Rate this"),
        swarm(
            vec![mock_agent(MockOutputMode::JsonSchema, 1, None, None, None)],
            equal_weights(),
        ),
        responses_text(vec!["Only option"]),
    );
    let msg = post_expect_error_msg(request).await;
    assert!(
        msg.contains("expected two or more") && msg.contains("got 1"),
        "unexpected error: {msg}"
    );
}

/// All agents have count=0 → InvalidSwarm.
#[tokio::test]
async fn test_error_invalid_swarm_all_count_zero() {
    let request = params(
        42,
        user_text("Compare"),
        swarm(
            vec![
                mock_agent(MockOutputMode::Instruction, 0, None, None, None),
                mock_agent(MockOutputMode::ToolCall, 0, None, None, None),
            ],
            weights(vec![Decimal::new(5, 1), Decimal::new(5, 1)]),
        ),
        responses_text(vec!["A", "B"]),
    );
    let msg = post_expect_error_msg(request).await;
    assert!(
        msg.contains("invalid_swarm") && msg.contains("1 and 128"),
        "unexpected error: {msg}"
    );
}

/// Empty agents vec → InvalidSwarm.
#[tokio::test]
async fn test_error_invalid_swarm_empty_agents() {
    let request = params(
        42,
        user_text("Which is better?"),
        objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai_sdk::swarm::InlineSwarmBase {
                agents: vec![],
                weights: Some(objectiveai_sdk::Weights::Weights(vec![])),
            },
        ),
        responses_text(vec!["X", "Y"]),
    );
    let msg = post_expect_error_msg(request).await;
    assert!(
        msg.contains("at least one positive value"),
        "unexpected error: {msg}"
    );
}

/// Profile length doesn't match agents length → InvalidSwarm.
#[tokio::test]
async fn test_error_invalid_swarm_profile_length_mismatch() {
    let request = params(
        42,
        user_text("Choose"),
        swarm(
            vec![
                mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                mock_agent(MockOutputMode::JsonSchema, 1, None, None, None),
            ],
            equal_weights(),
        ),
        responses_text(vec!["A", "B"]),
    );
    let msg = post_expect_error_msg(request).await;
    assert!(
        msg.contains("does not match") && msg.contains("weights length"),
        "unexpected error: {msg}"
    );
}

/// Duplicate agents with conflicting invert flags → InvalidSwarm.
#[tokio::test]
async fn test_error_invalid_swarm_conflicting_invert() {
    let request = params(
        42,
        user_text("Rank these"),
        swarm(
            vec![
                mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                mock_agent(MockOutputMode::Instruction, 1, None, None, None),
            ],
            objectiveai_sdk::Weights::Entries(vec![
                objectiveai_sdk::WeightsEntry { weight: Decimal::new(5, 1), invert: Some(false) },
                objectiveai_sdk::WeightsEntry { weight: Decimal::new(5, 1), invert: Some(true) },
            ]),
        ),
        responses_text(vec!["A", "B"]),
    );
    let msg = post_expect_error_msg(request).await;
    assert!(
        msg.contains("conflicting invert"),
        "unexpected error: {msg}"
    );
}

/// All weights are zero → error during swarm conversion.
#[tokio::test]
async fn test_error_invalid_profile_all_zero_weights() {
    let request = params(
        42,
        user_text("Score these"),
        swarm(
            vec![mock_agent(MockOutputMode::ToolCall, 1, None, None, None)],
            objectiveai_sdk::Weights::Weights(vec![Decimal::ZERO]),
        ),
        responses_text(vec!["A", "B"]),
    );
    let msg = post_expect_error_msg(request).await;
    assert!(msg.contains("at least one positive"), "unexpected error: {msg}");
}

// ---------------------------------------------------------------------------
// Logprobs tests
// ---------------------------------------------------------------------------

/// JsonSchema output mode with logprobs, 2 agents, 3 responses.
#[tokio::test]
async fn test_logprobs_json_schema_2_agents_seed_42() {
    let request = params(
        42,
        user_text("Rate these options"),
        swarm(
            vec![
                mock_agent(MockOutputMode::JsonSchema, 1, Some(5), None, None),
                mock_agent(MockOutputMode::JsonSchema, 1, Some(5), None, None),
            ],
            weights(vec![Decimal::new(6, 1), Decimal::new(4, 1)]),
        ),
        responses_text(vec!["Option A", "Option B", "Option C"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.scores.len(), 3);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_json_schema_2_agents_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/logprobs_json_schema_2_agents_seed_42.json"),
    );
}

/// JsonSchema, 3 agents with unequal weights, 4 responses, high top_logprobs.
#[tokio::test]
async fn test_logprobs_json_schema_3_agents_unequal_seed_77() {
    let request = params(
        77,
        user_text("Rank these candidates"),
        swarm(
            vec![
                mock_agent(MockOutputMode::JsonSchema, 1, Some(10), None, None),
                mock_agent(MockOutputMode::JsonSchema, 1, Some(10), None, None),
                mock_agent(MockOutputMode::JsonSchema, 1, Some(10), None, None),
            ],
            weights(vec![Decimal::new(5, 1), Decimal::new(3, 1), Decimal::new(2, 1)]),
        ),
        responses_text(vec![
            "Candidate Alpha", "Candidate Beta", "Candidate Gamma", "Candidate Delta",
        ]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.scores.len(), 4);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_json_schema_3_agents_unequal_seed_77.json"),
        include_str!("../../assets/vector/completions/client_tests/logprobs_json_schema_3_agents_unequal_seed_77.json"),
    );
}

/// ToolCall output mode with logprobs, single agent.
#[tokio::test]
async fn test_logprobs_tool_call_single_agent_seed_55() {
    let request = params(
        55,
        user_text("Pick the best tool"),
        swarm(
            vec![mock_agent(MockOutputMode::ToolCall, 1, Some(3), None, None)],
            equal_weights(),
        ),
        responses_text(vec!["Hammer", "Screwdriver"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.scores.len(), 2);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_tool_call_single_agent_seed_55.json"),
        include_str!("../../assets/vector/completions/client_tests/logprobs_tool_call_single_agent_seed_55.json"),
    );
}

/// Error primary agent with healthy logprobs-enabled fallback.
#[tokio::test]
async fn test_logprobs_error_with_fallback_seed_99() {
    let request = params(
        99,
        user_text("Score these"),
        swarm(
            vec![mock_agent(
                MockOutputMode::JsonSchema, 1, Some(8), Some(true),
                Some(vec![objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase {
                    upstream: MockUpstream::Mock,
                    output_mode: MockOutputMode::JsonSchema,
                    top_logprobs: Some(8),
                    error: None,
                    error_probability: None,
                    mcp_servers: None,
                    laboratories: None,
                    objectiveai_mcp: None,
                    plugins: Vec::new(),
                    calls: None,
                })]),
            )],
            equal_weights(),
        ),
        responses_text(vec!["Plan A", "Plan B", "Plan C"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.scores.len(), 3);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_error_with_fallback_seed_99.json"),
        include_str!("../../assets/vector/completions/client_tests/logprobs_error_with_fallback_seed_99.json"),
    );
}

/// Both primary and fallback error — should produce error completion, no votes.
#[tokio::test]
async fn test_logprobs_all_errors_seed_42() {
    let request = params(
        42,
        user_text("Evaluate"),
        swarm(
            vec![mock_agent(
                MockOutputMode::JsonSchema, 1, Some(5), Some(true),
                Some(vec![objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase {
                    upstream: MockUpstream::Mock,
                    output_mode: MockOutputMode::ToolCall,
                    top_logprobs: Some(3),
                    error: Some(true),
                    error_probability: None,
                    mcp_servers: None,
                    laboratories: None,
                    objectiveai_mcp: None,
                    plugins: Vec::new(),
                    calls: None,
                })]),
            )],
            equal_weights(),
        ),
        responses_text(vec!["X", "Y"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_all_errors_seed_42.json"),
        include_str!("../../assets/vector/completions/client_tests/logprobs_all_errors_seed_42.json"),
    );
}

/// Instruction output mode with logprobs (rare combination).
#[tokio::test]
async fn test_logprobs_instruction_seed_33() {
    let request = params(
        33,
        user_text("Which do you prefer?"),
        swarm(
            vec![mock_agent(MockOutputMode::Instruction, 1, Some(2), None, None)],
            equal_weights(),
        ),
        responses_text(vec!["Cats", "Dogs"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.scores.len(), 2);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_instruction_seed_33.json"),
        include_str!("../../assets/vector/completions/client_tests/logprobs_instruction_seed_33.json"),
    );
}

/// Mixed: response_format + tool_call + instruction agents, one with error+fallback.
#[tokio::test]
async fn test_logprobs_mixed_modes_with_fallback_seed_88() {
    let request = params(
        88,
        user_text("Compare these designs"),
        swarm(
            vec![
                mock_agent(MockOutputMode::JsonSchema, 1, Some(6), None, None),
                mock_agent(MockOutputMode::ToolCall, 1, Some(4), None, None),
                mock_agent(
                    MockOutputMode::Instruction, 1, Some(3), Some(true),
                    Some(vec![objectiveai_sdk::agent::InlineAgentBase::Mock(MockAgentBase {
                        upstream: MockUpstream::Mock,
                        output_mode: MockOutputMode::Instruction,
                        top_logprobs: Some(3),
                        error: None,
                        error_probability: None,
                        mcp_servers: None,
                        laboratories: None,
                        objectiveai_mcp: None,
                        plugins: Vec::new(),
                        calls: None,
                    })]),
                ),
            ],
            weights(vec![Decimal::new(4, 1), Decimal::new(4, 1), Decimal::new(2, 1)]),
        ),
        responses_text(vec!["Design Minimal", "Design Ornate", "Design Hybrid"]),
    );
    let stream = post_streaming(request).await;
    let result = normalize(run_and_check(stream).await);
    assert_eq!(result.scores.len(), 3);
    assert!(result.completions.len() >= 3);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_mixed_modes_with_fallback_seed_88.json"),
        include_str!("../../assets/vector/completions/client_tests/logprobs_mixed_modes_with_fallback_seed_88.json"),
    );
}
