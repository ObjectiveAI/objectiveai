//! Integration tests for the `/laboratories/executions` endpoint of
//! the spawned `objectiveai-api` server. The test harness sets
//! `LABORATORY_USE_MOCK_ORCHESTRATOR=1` so the server picks the mock
//! orchestrator at startup — no Docker daemon required.

type Params = objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams;
type LaboratoryExecution =
    objectiveai_sdk::laboratories::executions::response::unary::LaboratoryExecution;
type LaboratoryExecutionChunk =
    objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk;

use crate::common;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn builder_agent(
    seed_error: bool,
    error_probability: Option<u8>,
) -> objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional {
    objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
        objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
            inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase {
                mode: Some(objectiveai_sdk::agent::mock::Mode::LaboratoryBuilder),
                error: if seed_error { Some(true) } else { None },
                error_probability,
                ..Default::default()
            }),
            fallbacks: None,
        },
    )
}

fn evaluation_agent() -> objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional {
    objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
        objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
            inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase {
                mode: Some(objectiveai_sdk::agent::mock::Mode::LaboratoryEvaluation),
                ..Default::default()
            }),
            fallbacks: None,
        },
    )
}

fn user_message(text: &str) -> objectiveai_sdk::agent::completions::message::Message {
    objectiveai_sdk::agent::completions::message::Message::User(
        objectiveai_sdk::agent::completions::message::UserMessage {
            content: objectiveai_sdk::agent::completions::message::RichContent::Text(text.to_string()),
            name: None,
        },
    )
}

fn string_schema() -> objectiveai_sdk::functions::expression::InputSchema {
    objectiveai_sdk::functions::expression::InputSchema::String(
        objectiveai_sdk::functions::expression::StringInputSchema {
            r#type: objectiveai_sdk::functions::expression::StringInputSchemaType::String,
            description: None,
            r#enum: None,
        },
    )
}

fn make_request(
    builder_agents: Vec<objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional>,
    eval: bool,
    seed: i64,
) -> Params {
    Params {
        docker_image: "alpine:3.23.3".to_string(),
        builder_agents,
        evaluation_agent: if eval { Some(evaluation_agent()) } else { None },
        builder_messages: vec![user_message("Build something.")],
        evaluation_messages: if eval {
            Some(vec![user_message("Evaluate the output.")])
        } else {
            None
        },
        evaluation_output_schema: if eval { Some(string_schema()) } else { None },
        builder_continuation: None,
        evaluation_continuation: None,
        max_evaluation_retries: Some(1),
        persist: Some(false),
        provider: None,
        seed: Some(seed),
        stream: Some(true),
    }
}

fn check_created(expected: &std::cell::Cell<Option<u64>>, _i: usize, created: u64) {
    match expected.get() {
        None => expected.set(Some(created)),
        Some(e) => assert_eq!(e, created, "created timestamp changed mid-stream"),
    }
}

async fn post_streaming(
    params: Params,
) -> impl futures::Stream<Item = LaboratoryExecutionChunk> + Unpin {
    use futures::StreamExt;
    let http = common::server::client();
    let stream = http
        .send_streaming::<LaboratoryExecutionChunk, _, _>(
            reqwest::Method::POST,
            "/laboratories/executions",
            Some(params),
        )
        .await
        .expect("send_streaming should succeed");
    Box::pin(stream.map(|item| match item {
        Ok(chunk) => chunk,
        Err(e) => panic!("chunk deserialize / stream error: {e:?}"),
    }))
}

async fn run_execution(params: Params) -> LaboratoryExecution {
    let stream = post_streaming(params).await;
    let expected_created = std::cell::Cell::new(None);
    let agg = common::stream_harness::consume_stream(
        stream,
        |agg, c| agg.push(c),
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(
                chunk.usage.is_none(),
                "chunk {i} (non-final) has usage, expected None",
            );
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(
                chunk.usage.is_none(),
                "chunk {i} (second-to-last) has usage, expected None",
            );
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(
                chunk.usage.is_some(),
                "final chunk {i} has no usage, expected Some",
            );
        },
    )
    .await;
    LaboratoryExecution::from(agg)
}

fn normalize(mut exec: LaboratoryExecution) -> LaboratoryExecution {
    exec.normalize_for_tests();
    exec
}

fn assert_snapshot(json: &str, path: &str, expected: &str) {
    common::stream_harness::assert_snapshot(
        json,
        path,
        expected,
        "UPDATE_LABORATORIES_EXECUTIONS_LOCAL_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn single_builder_no_eval_seed_42() {
    let request = make_request(vec![builder_agent(false, None)], false, 42);
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/laboratories/executions/local/client_tests/single_builder_no_eval_seed_42.json"
        ),
        include_str!(
            "../../assets/laboratories/executions/local/client_tests/single_builder_no_eval_seed_42.json"
        ),
    );
}

#[tokio::test]
async fn single_builder_with_eval_seed_42() {
    let request = make_request(vec![builder_agent(false, None)], true, 42);
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/laboratories/executions/local/client_tests/single_builder_with_eval_seed_42.json"
        ),
        include_str!(
            "../../assets/laboratories/executions/local/client_tests/single_builder_with_eval_seed_42.json"
        ),
    );
}

#[tokio::test]
async fn two_builders_with_eval_seed_99() {
    let request = make_request(
        vec![builder_agent(false, None), builder_agent(false, None)],
        true,
        99,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/laboratories/executions/local/client_tests/two_builders_with_eval_seed_99.json"
        ),
        include_str!(
            "../../assets/laboratories/executions/local/client_tests/two_builders_with_eval_seed_99.json"
        ),
    );
}

#[tokio::test]
async fn builder_error_50_with_eval_seed_10() {
    let request = make_request(vec![builder_agent(true, Some(50))], true, 10);
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/laboratories/executions/local/client_tests/builder_error_50_with_eval_seed_10.json"
        ),
        include_str!(
            "../../assets/laboratories/executions/local/client_tests/builder_error_50_with_eval_seed_10.json"
        ),
    );
}

#[tokio::test]
async fn two_builders_one_error_50_no_eval_seed_7() {
    let request = make_request(
        vec![builder_agent(false, None), builder_agent(true, Some(50))],
        false,
        7,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/laboratories/executions/local/client_tests/two_builders_one_error_50_no_eval_seed_7.json"
        ),
        include_str!(
            "../../assets/laboratories/executions/local/client_tests/two_builders_one_error_50_no_eval_seed_7.json"
        ),
    );
}
