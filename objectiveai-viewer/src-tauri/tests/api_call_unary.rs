//! Unary `api_call` flow snapshot test.
//!
//! Drives `api_call_run_impl` against `POST /agent/completions` with
//! a default mock agent and `stream: false`. The mock agent runs
//! entirely in-process inside the test API server — no upstream
//! network traffic, fully deterministic per seed. The envelope
//! sequence is always `{Begin, Chunk, End}` for unary endpoints, so
//! the snapshot is three events: Begin, one Chunk carrying the full
//! `AgentCompletion` response body, End.

mod common;

use std::time::Duration;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional as Agent;
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::mock::AgentBase as MockAgentBase;
use objectiveai_sdk::viewer::ApiCallSubType;

use common::{ViewerTestEnv, is_api_call_end, snapshot, test_api_address};

const SNAPSHOT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots/api_call_unary_agent_completions_mock_seed_42.jsonl");

#[tokio::test]
async fn api_call_unary_agent_completions_mock_seed_42() {
    if test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping");
        return;
    }
    let mut env = ViewerTestEnv::new();

    let params = AgentCompletionCreateParams {
        messages: vec![],
        agent: Agent::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(
                    MockAgentBase::default(),
                ),
                fallbacks: None,
            },
        ),
        provider: None,
        response_format: None,
        seed: Some(42),
        stream: Some(false),
        continuation: None,
    };
    let body = serde_json::to_value(&params).expect("serialize params");

    objectiveai_viewer::test_internals::api_call_run_impl(
        env.events_tx.clone(),
        env.http_client.clone(),
        ApiCallSubType::PostAgentCompletions,
        body,
        "test-iframe".to_string(),
    )
    .await
    .expect("api_call_run_impl returned an error");

    let events = env
        .drain_until_end(is_api_call_end, Duration::from_secs(30))
        .await;

    let actual = snapshot::events_to_jsonl(&events);
    snapshot::assert_snapshot(
        &actual,
        SNAPSHOT_PATH,
        include_str!("snapshots/api_call_unary_agent_completions_mock_seed_42.jsonl"),
    );
}
