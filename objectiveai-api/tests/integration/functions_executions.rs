//! Integration tests for the `/functions/executions` endpoint of the
//! spawned `objectiveai-api` server. Each test POSTs a
//! `FunctionExecutionCreateParams` body, streams the SSE response, and
//! snapshots the aggregated `FunctionExecution`.

#![allow(clippy::too_many_arguments)]

use futures::StreamExt;
use rust_decimal::Decimal;

use objectiveai::functions::executions::request::{
    FunctionExecutionCreateParams, Strategy,
};
use objectiveai::functions::executions::response::streaming::FunctionExecutionChunk;
use objectiveai::functions::executions::response::unary::FunctionExecution;
use objectiveai::functions::expression::InputValue;

use crate::common;

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

fn make_request(
    function_repo: &str,
    profile_repo: &str,
    input: InputValue,
    seed: i64,
) -> FunctionExecutionCreateParams {
    FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: function_repo.to_string(),
            },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: profile_repo.to_string(),
            },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input,
        split: None,
        invert: None,
        provider: None,
        seed: Some(seed),
        stream: Some(true),
        continuation: None,
    }
}

fn make_request_with_overrides(
    function_repo: &str,
    profile_repo: &str,
    overrides: impl FnOnce(&mut FunctionExecutionCreateParams),
) -> FunctionExecutionCreateParams {
    let mut params = make_request(
        function_repo,
        profile_repo,
        InputValue::Object(indexmap::indexmap! {}),
        42,
    );
    overrides(&mut params);
    params
}

// ---------------------------------------------------------------------------
// Streaming + aggregation helpers
// ---------------------------------------------------------------------------

fn check_created(expected: &std::cell::Cell<Option<u64>>, i: usize, created: u64) {
    match expected.get() {
        None => expected.set(Some(created)),
        Some(exp) => assert_eq!(created, exp, "chunk {i} has created {created}, expected {exp}"),
    }
}

fn check_id(expected: &std::cell::RefCell<Option<String>>, i: usize, id: &str) {
    let mut exp = expected.borrow_mut();
    match &*exp {
        None => *exp = Some(id.to_string()),
        Some(exp) => assert_eq!(id, exp, "chunk {i} has id {id:?}, expected {exp:?}"),
    }
}

/// POST `params` to `/functions/executions` (streaming) on the spawned
/// api server and return the resulting `Stream<FunctionExecutionChunk>`.
async fn post_streaming(
    params: FunctionExecutionCreateParams,
) -> impl futures::Stream<Item = FunctionExecutionChunk> + Unpin {
    let http = common::server::client();
    let stream = http
        .send_streaming::<FunctionExecutionChunk, _, _>(
            reqwest::Method::POST,
            "/functions/executions",
            Some(params),
        )
        .await
        .expect("send_streaming should succeed");
    Box::pin(stream.map(|item| match item {
        Ok(chunk) => chunk,
        Err(e) => panic!("chunk deserialize / stream error: {e:?}"),
    }))
}

/// POST `params` and assert the request fails with the given HTTP status
/// code. The error may surface either as a non-2xx HTTP response from
/// `send_streaming` or as an in-stream error chunk; both paths return a
/// debug string that contains the structured error body so callers can
/// match on the inner `kind`.
async fn post_expect_err_kind(
    params: FunctionExecutionCreateParams,
    expected_status: u16,
) -> String {
    let http = common::server::client();
    let result = http
        .send_streaming::<FunctionExecutionChunk, _, _>(
            reqwest::Method::POST,
            "/functions/executions",
            Some(params),
        )
        .await;
    let mut stream = match result {
        Ok(s) => Box::pin(s),
        Err(e) => {
            let dbg = format!("{e:?}");
            assert!(
                dbg.contains(&format!("code: {expected_status}"))
                    || dbg.contains(&format!("BadStatus {{ code: {expected_status}")),
                "expected status {expected_status}, got: {dbg}",
            );
            return dbg;
        }
    };
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                if let Some(err) = &chunk.error {
                    let dbg = format!("{err:?}");
                    assert_eq!(
                        err.code, expected_status,
                        "expected status {expected_status}, got: {dbg}",
                    );
                    return dbg;
                }
            }
            Err(e) => {
                let dbg = format!("{e:?}");
                assert!(
                    dbg.contains(&format!("code: {expected_status}"))
                        || dbg.contains(&format!("BadStatus {{ code: {expected_status}")),
                    "expected status {expected_status}, got: {dbg}",
                );
                return dbg;
            }
        }
    }
    panic!("expected an error, but stream ended without one");
}

async fn run_execution(params: FunctionExecutionCreateParams) -> FunctionExecution {
    let stream = post_streaming(params).await;
    let expected_created = std::cell::Cell::new(None);
    let expected_id = std::cell::RefCell::new(None);
    let agg = common::stream_harness::consume_stream(
        stream,
        |agg, c| agg.push(c),
        |i, chunk| {
            check_id(&expected_id, i, &chunk.id);
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.tasks.len() <= 1, "chunk {i} has {} tasks, expected at most 1", chunk.tasks.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_id(&expected_id, i, &chunk.id);
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.tasks.len() <= 1, "chunk {i} has {} tasks, expected at most 1", chunk.tasks.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_id(&expected_id, i, &chunk.id);
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
        },
    ).await;
    FunctionExecution::from(agg)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum IndexKey {
    Split(u64),
    Swiss { pool: u64, round: u64 },
}

impl std::fmt::Display for IndexKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexKey::Split(i) => write!(f, "split_index={i}"),
            IndexKey::Swiss { pool, round } => write!(f, "(swiss_pool_index={pool}, swiss_round={round})"),
        }
    }
}

fn check_indexed_task(
    i: usize,
    chunk: &FunctionExecutionChunk,
    key_to_id: &std::cell::RefCell<std::collections::HashMap<IndexKey, String>>,
    extract_key: impl Fn(&objectiveai::functions::executions::response::streaming::FunctionExecutionTaskChunk) -> IndexKey,
) {
    assert_eq!(
        chunk.tasks.len(),
        1,
        "chunk {i} has {} tasks, expected exactly 1",
        chunk.tasks.len(),
    );
    let task = match &chunk.tasks[0] {
        objectiveai::functions::executions::response::streaming::TaskChunk::FunctionExecution(t) => t,
        other => panic!("chunk {i} task[0] is not a FunctionExecution task chunk: {other:?}"),
    };
    let key = extract_key(task);
    let inner_id = task.inner.id.clone();
    let mut map = key_to_id.borrow_mut();
    match map.get(&key) {
        Some(existing) => {
            assert_eq!(
                existing, &inner_id,
                "chunk {i} key {key} inner response_id changed from {existing:?} to {inner_id:?}",
            );
        }
        None => {
            for (other_key, other_id) in map.iter() {
                assert_ne!(
                    other_id, &inner_id,
                    "chunk {i} key {key} uses inner response_id {inner_id:?} already bound to key {other_key}",
                );
            }
            map.insert(key, inner_id);
        }
    }
}

async fn run_execution_split(params: FunctionExecutionCreateParams) -> FunctionExecution {
    let stream = post_streaming(params).await;
    let expected_created = std::cell::Cell::new(None);
    let expected_id = std::cell::RefCell::new(None);
    let key_to_id: std::cell::RefCell<std::collections::HashMap<IndexKey, String>> =
        std::cell::RefCell::new(std::collections::HashMap::new());

    let check_nonterminal = |i: usize, chunk: &FunctionExecutionChunk| {
        check_id(&expected_id, i, &chunk.id);
        check_created(&expected_created, i, chunk.created);
        assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        assert!(chunk.output.is_none(), "chunk {i} (non-final) has output, expected None");
        check_indexed_task(i, chunk, &key_to_id, |task| {
            let split = task.split_index.expect("non-terminal split chunk must have split_index set");
            assert!(task.swiss_pool_index.is_none(), "split task chunk has swiss_pool_index set");
            assert!(task.swiss_round.is_none(), "split task chunk has swiss_round set");
            IndexKey::Split(split)
        });
    };

    let agg = common::stream_harness::consume_stream(
        stream,
        |agg, c| agg.push(c),
        &check_nonterminal,
        &check_nonterminal,
        |i, chunk| {
            check_id(&expected_id, i, &chunk.id);
            check_created(&expected_created, i, chunk.created);
            assert_eq!(
                chunk.tasks.len(), 0,
                "terminal chunk {i} has {} tasks, expected 0",
                chunk.tasks.len(),
            );
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
        },
    ).await;
    FunctionExecution::from(agg)
}

async fn run_execution_swiss(params: FunctionExecutionCreateParams) -> FunctionExecution {
    let stream = post_streaming(params).await;
    let expected_created = std::cell::Cell::new(None);
    let expected_id = std::cell::RefCell::new(None);
    let key_to_id: std::cell::RefCell<std::collections::HashMap<IndexKey, String>> =
        std::cell::RefCell::new(std::collections::HashMap::new());

    let check_nonterminal = |i: usize, chunk: &FunctionExecutionChunk| {
        check_id(&expected_id, i, &chunk.id);
        check_created(&expected_created, i, chunk.created);
        assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        assert!(chunk.output.is_none(), "chunk {i} (non-final) has output, expected None");
        check_indexed_task(i, chunk, &key_to_id, |task| {
            let pool = task.swiss_pool_index.expect("non-terminal swiss chunk must have swiss_pool_index set");
            let round = task.swiss_round.expect("non-terminal swiss chunk must have swiss_round set");
            assert!(task.split_index.is_none(), "swiss task chunk has split_index set");
            IndexKey::Swiss { pool, round }
        });
    };

    let agg = common::stream_harness::consume_stream(
        stream,
        |agg, c| agg.push(c),
        &check_nonterminal,
        &check_nonterminal,
        |i, chunk| {
            check_id(&expected_id, i, &chunk.id);
            check_created(&expected_created, i, chunk.created);
            assert_eq!(
                chunk.tasks.len(), 0,
                "terminal chunk {i} has {} tasks, expected 0",
                chunk.tasks.len(),
            );
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
        },
    ).await;
    FunctionExecution::from(agg)
}

async fn run_execution_allow_error(params: FunctionExecutionCreateParams) -> FunctionExecution {
    run_execution(params).await
}

// ---------------------------------------------------------------------------
// Snapshot helpers
// ---------------------------------------------------------------------------

fn normalize(mut fe: FunctionExecution) -> FunctionExecution {
    fe.normalize_for_tests();
    fe
}

fn assert_snapshot(json: &str, path: &str, expected: &str) {
    common::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_FUNCTIONS_EXECUTIONS_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mock_1_scalar_leaf_binary_seed_42() {
    let request = make_request(
        "binary-classifier",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Hello world".into()),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_1_scalar_leaf_binary_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_1_scalar_leaf_binary_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_2_scalar_skip_false_seed_42() {
    let request = make_request(
        "spam-with-optional-sentiment",
        "instruction-and-schema",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Buy cheap watches now!!!".into()),
            "include_sentiment".into() => InputValue::Boolean(false),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_2_scalar_skip_false_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_2_scalar_skip_false_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_2_scalar_skip_true_seed_42() {
    let request = make_request(
        "spam-with-optional-sentiment",
        "instruction-and-schema",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("I love this product!".into()),
            "include_sentiment".into() => InputValue::Boolean(true),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_2_scalar_skip_true_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_2_scalar_skip_true_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_3_scalar_5way_seed_42() {
    let request = make_request(
        "five-star-rating",
        "triple-mode",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("The food was amazing".into()),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_3_scalar_5way_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_3_scalar_5way_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_4_vector_ranker_seed_42() {
    let request = make_request(
        "item-ranker",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Apple".into()),
                InputValue::String("Banana".into()),
                InputValue::String("Cherry".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_4_vector_ranker_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_4_vector_ranker_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_5_vector_context_multi_task_seed_42() {
    let request = make_request(
        "contextual-ranker",
        "contextual-duo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("best fruit".into()),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Apple".into()),
                InputValue::String("Banana".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_5_vector_context_multi_task_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_5_vector_context_multi_task_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_6_scalar_system_message_seed_42() {
    let request = make_request(
        "email-importance",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "subject".into() => InputValue::String("Meeting tomorrow".into()),
            "body".into() => InputValue::String("Don't forget the meeting at 3pm.".into()),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_6_scalar_system_message_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_6_scalar_system_message_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_7_vector_5_criteria_seed_42() {
    let request = make_request(
        "five-criteria-ranker",
        "schema-heavy-trio",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Option A".into()),
                InputValue::String("Option B".into()),
                InputValue::String("Option C".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_7_vector_5_criteria_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_7_vector_5_criteria_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_8_vector_context_skip_false_seed_42() {
    let request = make_request(
        "strict-contextual-ranker",
        "logprobs-and-tool",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("best answer".into()),
                "strict".into() => InputValue::Boolean(false),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Answer 1".into()),
                InputValue::String("Answer 2".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_8_vector_context_skip_false_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_8_vector_context_skip_false_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_8_vector_context_skip_true_seed_42() {
    let request = make_request(
        "strict-contextual-ranker",
        "logprobs-and-tool",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("best answer".into()),
                "strict".into() => InputValue::Boolean(true),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Answer 1".into()),
                InputValue::String("Answer 2".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_8_vector_context_skip_true_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_8_vector_context_skip_true_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_9_scalar_branch_2_tasks_seed_42() {
    let request = make_request(
        "spam-importance-branch",
        "schema-logprobs-solo",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Important project update".into()),
            "subject".into() => InputValue::String("Project update".into()),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_9_scalar_branch_2_tasks_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_9_scalar_branch_2_tasks_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_10_scalar_branch_3_tasks_error_seed_42() {
    let request = make_request(
        "triple-classifier-branch",
        "trio-with-error-agent",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Great service!".into()),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_10_scalar_branch_3_tasks_error_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_10_scalar_branch_3_tasks_error_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_11_scalar_branch_skip_false_seed_42() {
    let request = make_request(
        "classifier-with-optional-sentiment",
        "tool-and-schema",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Check this out".into()),
            "include_sentiment".into() => InputValue::Boolean(false),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_11_scalar_branch_skip_false_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_11_scalar_branch_skip_false_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_11_scalar_branch_skip_true_seed_42() {
    let request = make_request(
        "classifier-with-optional-sentiment",
        "tool-and-schema",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Check this out".into()),
            "include_sentiment".into() => InputValue::Boolean(true),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_11_scalar_branch_skip_true_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_11_scalar_branch_skip_true_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_12_vector_branch_2_vector_seed_42() {
    let request = make_request(
        "dual-ranker-branch",
        "logprobs-duo",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Red".into()),
                InputValue::String("Blue".into()),
                InputValue::String("Green".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_12_vector_branch_2_vector_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_12_vector_branch_2_vector_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_13_vector_branch_mixed_seed_42() {
    let request = make_request(
        "mixed-scalar-vector-branch",
        "schema-solo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("favorite color".into()),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Red".into()),
                InputValue::String("Blue".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_13_vector_branch_mixed_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_13_vector_branch_mixed_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_14_vector_branch_skip_false_seed_42() {
    let request = make_request(
        "ranker-with-optional-quality",
        "trio-with-error-instruction",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("rank these".into()),
                "include_quality".into() => InputValue::Boolean(false),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("X".into()),
                InputValue::String("Y".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_14_vector_branch_skip_false_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_14_vector_branch_skip_false_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_14_vector_branch_skip_true_seed_42() {
    let request = make_request(
        "ranker-with-optional-quality",
        "trio-with-error-instruction",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("rank these".into()),
                "include_quality".into() => InputValue::Boolean(true),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("X".into()),
                InputValue::String("Y".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_14_vector_branch_skip_true_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_14_vector_branch_skip_true_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_15_vector_branch_3_vector_logprobs_seed_42() {
    let request = make_request(
        "triple-ranker-branch",
        "high-logprobs-duo",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_15_vector_branch_3_vector_logprobs_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_15_vector_branch_3_vector_logprobs_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_16_vector_branch_4_tasks_error_logprobs_seed_42() {
    let request = make_request(
        "four-way-vector-branch",
        "quad-with-error",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("Evaluate these options".into()),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("First".into()),
                InputValue::String("Second".into()),
                InputValue::String("Third".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_16_vector_branch_4_tasks_error_logprobs_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_16_vector_branch_4_tasks_error_logprobs_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_17_vector_branch_mixed_skip_false_seed_42() {
    let request = make_request(
        "deep-optional-mixed-branch",
        "max-logprobs-duo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("compare these".into()),
                "deep".into() => InputValue::Boolean(false),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Foo".into()),
                InputValue::String("Bar".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_17_vector_branch_mixed_skip_false_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_17_vector_branch_mixed_skip_false_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_17_vector_branch_mixed_skip_true_seed_42() {
    let request = make_request(
        "deep-optional-mixed-branch",
        "max-logprobs-duo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("compare these".into()),
                "deep".into() => InputValue::Boolean(true),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Foo".into()),
                InputValue::String("Bar".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_17_vector_branch_mixed_skip_true_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_17_vector_branch_mixed_skip_true_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_18_scalar_super_branch_seed_42() {
    let request = make_request(
        "nested-scalar-super-branch",
        "expanded-nested-scalar",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Hello world".into()),
            "subject".into() => InputValue::String("greeting".into()),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_18_scalar_super_branch_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_18_scalar_super_branch_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_19_scalar_super_branch_skip_false_seed_42() {
    let request = make_request(
        "skipable-nested-scalar-branch",
        "mixed-nested-with-skip",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Test input".into()),
            "subject".into() => InputValue::String("testing".into()),
            "thorough".into() => InputValue::Boolean(false),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_19_scalar_super_branch_skip_false_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_19_scalar_super_branch_skip_false_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_19_scalar_super_branch_skip_true_seed_42() {
    let request = make_request(
        "skipable-nested-scalar-branch",
        "mixed-nested-with-skip",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Test input".into()),
            "subject".into() => InputValue::String("testing".into()),
            "thorough".into() => InputValue::Boolean(true),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_19_scalar_super_branch_skip_true_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_19_scalar_super_branch_skip_true_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_20_vector_super_branch_seed_42() {
    let request = make_request(
        "nested-vector-super-branch",
        "nested-vector-inline-remote",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
                InputValue::String("Gamma".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_20_vector_super_branch_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_20_vector_super_branch_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_21_vector_super_branch_context_seed_42() {
    let request = make_request(
        "contextual-nested-vector-branch",
        "deep-nested-vector",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("rank these options".into()),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("One".into()),
                InputValue::String("Two".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_21_vector_super_branch_context_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_21_vector_super_branch_context_seed_42.json"),
    );
}

#[tokio::test]
async fn test_inline_scalar_placeholder_seed_42() {
    let request = FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Inline(
            objectiveai::functions::FullInlineFunction::Standard(
                objectiveai::functions::InlineFunction::Scalar {
                    tasks: vec![
                        objectiveai::functions::TaskExpression::PlaceholderScalarFunction(
                            objectiveai::functions::PlaceholderScalarFunctionTaskExpression {
                                input_schema: serde_json::from_value(serde_json::json!({
                                    "type": "object",
                                    "properties": { "text": { "type": "string" } },
                                    "required": ["text"]
                                })).unwrap(),
                                skip: None,
                                map: None,
                                input: objectiveai::functions::expression::WithExpression::Expression(
                                    objectiveai::functions::expression::Expression::Starlark(
                                        "{'text': input['text']}".to_string(),
                                    ),
                                ),
                                output: objectiveai::functions::expression::Expression::Special(
                                    objectiveai::functions::expression::Special::Output,
                                ),
                            },
                        ),
                        objectiveai::functions::TaskExpression::PlaceholderScalarFunction(
                            objectiveai::functions::PlaceholderScalarFunctionTaskExpression {
                                input_schema: serde_json::from_value(serde_json::json!({
                                    "type": "object",
                                    "properties": { "text": { "type": "string" } },
                                    "required": ["text"]
                                })).unwrap(),
                                skip: None,
                                map: None,
                                input: objectiveai::functions::expression::WithExpression::Expression(
                                    objectiveai::functions::expression::Expression::Starlark(
                                        "{'text': input['text']}".to_string(),
                                    ),
                                ),
                                output: objectiveai::functions::expression::Expression::Special(
                                    objectiveai::functions::expression::Special::Output,
                                ),
                            },
                        ),
                    ],
                },
            ),
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Inline(
            objectiveai::functions::InlineProfile::Auto(
                objectiveai::swarm::InlineSwarmBase {
                    agents: vec![
                        objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
                            count: 1,
                            inner: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
                                objectiveai::agent::InlineAgentBaseWithFallbacks {
                                    inner: objectiveai::agent::InlineAgentBase::Mock(
                                        objectiveai::agent::mock::AgentBase {
                                            upstream: objectiveai::agent::mock::Upstream::Mock,
                                            output_mode: objectiveai::agent::mock::OutputMode::Instruction,
                                            top_logprobs: None,
                                            error: None,
                                            error_probability: None,
                                            mode: None,
                                            mcp_servers: None,
                                        },
                                    ),
                                    fallbacks: None,
                                },
                            ),
                        },
                    ],
                    weights: Some(objectiveai::Weights::Weights(vec![Decimal::ONE])),
                },
            ),
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input: InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Hello world".into()),
        }),
        split: None,
        invert: None,
        provider: None,
        seed: Some(42),
        stream: Some(true),
        continuation: None,
    };
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/inline_scalar_placeholder_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/inline_scalar_placeholder_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_25_scalar_placeholder_remote_swarm_seed_42() {
    let request = make_request(
        "dual-placeholder",
        "schema-and-tool",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Hello world".into()),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_25_scalar_placeholder_remote_swarm_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_25_scalar_placeholder_remote_swarm_seed_42.json"),
    );
}

// ---------------------------------------------------------------------------
// SwissSystem strategy tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mock_4_vector_swiss_default_20_items_seed_7() {
    let mut request = make_request(
        "item-ranker",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array((0..20).map(|i| InputValue::String(format!("Item{i}"))).collect()),
        }),
        7,
    );
    request.strategy = Some(Strategy::SwissSystem { pool: None, rounds: None });
    let result = normalize(run_execution_swiss(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_4_vector_swiss_default_20_items_seed_7.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_4_vector_swiss_default_20_items_seed_7.json"),
    );
}

#[tokio::test]
async fn test_mock_5_vector_swiss_pool5_rounds3_20_items_seed_7() {
    let mut request = make_request(
        "contextual-ranker",
        "contextual-duo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("rank these items".into()),
            }),
            "items".into() => InputValue::Array((0..20).map(|i| InputValue::String(format!("Item{i}"))).collect()),
        }),
        7,
    );
    request.strategy = Some(Strategy::SwissSystem { pool: Some(5), rounds: Some(3) });
    let result = normalize(run_execution_swiss(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_5_vector_swiss_pool5_rounds3_20_items_seed_7.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_5_vector_swiss_pool5_rounds3_20_items_seed_7.json"),
    );
}

#[tokio::test]
async fn test_mock_7_vector_swiss_pool4_rounds3_20_items_seed_7() {
    let mut request = make_request(
        "five-criteria-ranker",
        "schema-heavy-trio",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array((0..20).map(|i| InputValue::String(format!("Item{i}"))).collect()),
        }),
        7,
    );
    request.strategy = Some(Strategy::SwissSystem { pool: Some(4), rounds: Some(3) });
    let result = normalize(run_execution_swiss(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_7_vector_swiss_pool4_rounds3_20_items_seed_7.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_7_vector_swiss_pool4_rounds3_20_items_seed_7.json"),
    );
}

// ---------------------------------------------------------------------------
// Mapped function tasks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mock_22_scalar_mapped_branch_2_items_seed_42() {
    let request = make_request(
        "mapped-branch-with-votes",
        "remote-swarm-mapped-branch",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_22_scalar_mapped_branch_2_items_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_22_scalar_mapped_branch_2_items_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_22_scalar_mapped_branch_2_items_seed_123() {
    let request = make_request(
        "mapped-branch-with-votes",
        "remote-swarm-mapped-branch",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
            ]),
        }),
        123,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_22_scalar_mapped_branch_2_items_seed_123.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_22_scalar_mapped_branch_2_items_seed_123.json"),
    );
}

#[tokio::test]
async fn test_mock_23_scalar_mapped_branch_3_items_seed_42() {
    let request = make_request(
        "mapped-branch-with-classifiers",
        "remote-swarm-classifiers",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("A".into()),
                InputValue::String("B".into()),
                InputValue::String("C".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_23_scalar_mapped_branch_3_items_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_23_scalar_mapped_branch_3_items_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_23_scalar_mapped_branch_2_items_seed_42() {
    let request = make_request(
        "mapped-branch-with-classifiers",
        "remote-swarm-classifiers",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("A".into()),
                InputValue::String("B".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_23_scalar_mapped_branch_2_items_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_23_scalar_mapped_branch_2_items_seed_42.json"),
    );
}

#[tokio::test]
async fn test_mock_24_scalar_mapped_branch_with_func_2_items_seed_42() {
    let request = make_request(
        "mapped-branch-mixed-tasks",
        "remote-swarm-classifiers",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_24_scalar_mapped_branch_with_func_2_items_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/mock_24_scalar_mapped_branch_with_func_2_items_seed_42.json"),
    );
}

// ===========================================================================
// Error tests
// ===========================================================================

#[tokio::test]
async fn test_error_1_1_invalid_retry_token() {
    let request = make_request_with_overrides(
        "binary-classifier",
        "solo-instruction",
        |p| {
            p.retry_token = Some("not-a-valid-retry-token!!!".to_string());
            p.input = InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("test".into()),
            });
        },
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("invalid_retry_token"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_1_3_scalar_function_swiss_strategy() {
    let request = make_request_with_overrides(
        "binary-classifier",
        "solo-instruction",
        |p| {
            p.strategy = Some(Strategy::SwissSystem { pool: None, rounds: None });
            p.input = InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("test".into()),
            });
        },
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("invalid_function_for_strategy"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_1_4_invalid_strategy_pool() {
    let request = make_request_with_overrides(
        "item-ranker",
        "solo-instruction",
        |p| {
            p.strategy = Some(Strategy::SwissSystem { pool: Some(1), rounds: Some(3) });
            p.input = InputValue::Object(indexmap::indexmap! {
                "items".into() => InputValue::Array(vec![
                    InputValue::String("A".into()),
                    InputValue::String("B".into()),
                    InputValue::String("C".into()),
                ]),
            });
        },
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("invalid_strategy"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_1_function_not_found() {
    let request = make_request("mock-nonexistent", "solo-instruction", InputValue::Object(indexmap::indexmap! {}), 42);
    let body = post_expect_err_kind(request, 404).await;
    assert!(body.contains("fetch_function") || body.contains("function_not_found"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_3_profile_not_found() {
    let request = make_request(
        "binary-classifier",
        "mock-nonexistent",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 404).await;
    assert!(body.contains("fetch_profile") || body.contains("profile_not_found"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_5_input_schema_mismatch() {
    let request = make_request(
        "binary-classifier",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "wrong_field".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("input_schema_mismatch"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_6_tasks_length_mismatch() {
    let request = make_request(
        "binary-classifier",
        "two-task-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("invalid_profile"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_7_weights_length_mismatch() {
    let request = make_request(
        "binary-classifier",
        "error-weights-length-mismatch",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("invalid_profile"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_8_placeholder_for_function_task() {
    let request = make_request(
        "spam-importance-branch",
        "placeholder-and-remote-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
            "subject".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("invalid_profile"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_17_bad_task_expression() {
    let request = make_request(
        "error-missing-input-key",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("invalid_expression"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_20_invalid_swarm() {
    let request = make_request(
        "binary-classifier",
        "error-weight-count-mismatch",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("invalid_swarm"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_21_recursive_function_not_found() {
    let request = make_request(
        "error-missing-sub-function",
        "baseline-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 404).await;
    assert!(body.contains("fetch_function") || body.contains("function_not_found"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_22_recursive_profile_not_found() {
    let request = make_request(
        "spam-importance-branch",
        "dangling-and-valid-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
            "subject".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 404).await;
    assert!(body.contains("fetch_profile") || body.contains("profile_not_found"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_23_circular_dependency_simple() {
    let request = make_request(
        "error-cycle-a",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("circular_dependency"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_24_circular_dependency_complex() {
    let request = make_request(
        "error-cycle-abc-a",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("circular_dependency"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_2_25_recursive_input_schema_mismatch() {
    let request = make_request(
        "error-wrong-sub-input",
        "baseline-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let body = post_expect_err_kind(request, 400).await;
    assert!(body.contains("input_schema_mismatch"), "unexpected error: {body}");
}

#[tokio::test]
async fn test_error_3_1_all_agents_error() {
    let request = make_request(
        "binary-classifier",
        "error-all-agents-fail",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(request).await;
    assert_eq!(result.tasks.len(), 1);
    match &result.tasks[0] {
        objectiveai::functions::executions::response::unary::Task::VectorCompletion(vt) => {
            assert!(vt.error.is_none(), "expected no task-level error, got: {:?}", vt.error);
            assert!(!vt.inner.completions.is_empty(), "expected at least one completion");
            for completion in &vt.inner.completions {
                assert!(
                    completion.inner.error.is_some(),
                    "expected error on agent completion, got None",
                );
            }
        }
        other => panic!("expected VectorCompletion task, got: {other:?}"),
    }
    assert!(
        matches!(&result.output.output, objectiveai::functions::expression::TaskOutputOwned::Scalar(s) if *s == rust_decimal::dec!(0.5)),
        "expected Scalar(0.5) fallback, got: {:?}",
        result.output,
    );
}

#[tokio::test]
async fn test_error_4_1_output_expression_fails() {
    let request = make_request(
        "error-bad-output-field",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

#[tokio::test]
async fn test_error_4_2_scalar_output_out_of_range() {
    let request = make_request(
        "error-scalar-out-of-range",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

#[tokio::test]
async fn test_error_4_3_scalar_got_vector() {
    let request = make_request(
        "error-scalar-returns-vector",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

#[tokio::test]
async fn test_error_4_4_vector_output_bad_sum() {
    let request = make_request(
        "error-vector-bad-sum",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("A".into()),
                InputValue::String("B".into()),
            ]),
        }),
        42,
    );
    let result = run_execution_allow_error(request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

#[tokio::test]
async fn test_error_4_5_vector_got_scalar() {
    let request = make_request(
        "error-vector-returns-scalar",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("A".into()),
                InputValue::String("B".into()),
            ]),
        }),
        42,
    );
    let result = run_execution_allow_error(request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

#[tokio::test]
async fn test_error_4_6_output_vectors_variant() {
    let request = make_request(
        "error-nested-list-output",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

#[tokio::test]
async fn test_error_4_7_output_returns_none() {
    let request = make_request(
        "error-none-output",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

#[tokio::test]
async fn test_error_6_1_reasoning_agent_error() {
    let request = make_request_with_overrides(
        "binary-classifier",
        "solo-instruction",
        |p| {
            p.reasoning = Some(objectiveai::functions::executions::request::Reasoning {
                agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                    objectiveai::agent::InlineAgentBaseWithFallbacks {
                        inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase {
                            upstream: objectiveai::agent::mock::Upstream::Mock,
                            output_mode: objectiveai::agent::mock::OutputMode::Instruction,
                            top_logprobs: None,
                            error: Some(true),
                            error_probability: None,
                            mode: None,
                            mcp_servers: None,
                        }),
                        fallbacks: None,
                    },
                ),
            });
            p.input = InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("test".into()),
            });
        },
    );
    let result = run_execution_allow_error(request).await;
    assert!(
        result.reasoning.as_ref().is_some_and(|r| r.error.is_some()),
        "expected reasoning error, got: {:?}",
        result.reasoning,
    );
}

// ===========================================================================
// Split tests
// ===========================================================================

#[tokio::test]
async fn test_split_scalar_binary_seed_42() {
    let request = FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: "binary-classifier".to_string(),
            },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: "solo-instruction".to_string(),
            },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input: InputValue::Array(vec![
            InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("Hello world".into()),
            }),
            InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("Buy cheap watches".into()),
            }),
            InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("Good morning".into()),
            }),
        ]),
        split: Some(true),
        invert: None,
        provider: None,
        seed: Some(42),
        stream: Some(true),
        continuation: None,
    };
    let result = normalize(run_execution_split(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/split_scalar_binary_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/split_scalar_binary_seed_42.json"),
    );
}

fn ten_tweet_swarm_two_agents(top_logprobs_first: Option<u64>, top_logprobs_second: Option<u64>, output_mode_first: objectiveai::agent::mock::OutputMode, output_mode_second: objectiveai::agent::mock::OutputMode, weights: Vec<Decimal>) -> objectiveai::swarm::InlineSwarmBase {
    objectiveai::swarm::InlineSwarmBase {
        agents: vec![
            objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
                count: 1,
                inner: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
                    objectiveai::agent::InlineAgentBaseWithFallbacks {
                        inner: objectiveai::agent::InlineAgentBase::Mock(
                            objectiveai::agent::mock::AgentBase {
                                upstream: objectiveai::agent::mock::Upstream::Mock,
                                output_mode: output_mode_first,
                                top_logprobs: top_logprobs_first,
                                error: None,
                                error_probability: None,
                                mode: None,
                                mcp_servers: None,
                            },
                        ),
                        fallbacks: None,
                    },
                ),
            },
            objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
                count: 1,
                inner: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
                    objectiveai::agent::InlineAgentBaseWithFallbacks {
                        inner: objectiveai::agent::InlineAgentBase::Mock(
                            objectiveai::agent::mock::AgentBase {
                                upstream: objectiveai::agent::mock::Upstream::Mock,
                                output_mode: output_mode_second,
                                top_logprobs: top_logprobs_second,
                                error: None,
                                error_probability: None,
                                mode: None,
                                mcp_servers: None,
                            },
                        ),
                        fallbacks: None,
                    },
                ),
            },
        ],
        weights: Some(objectiveai::Weights::Weights(weights)),
    }
}

#[tokio::test]
async fn test_split_tweet_scorer_10_tweets_seed_42() {
    let input: InputValue = serde_json::from_str(include_str!(
        "../../assets/functions/executions/client_tests/inputs/10_tweets.json"
    )).expect("10_tweets.json must parse as InputValue");
    let request = FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: "tweet-scorer".to_string(),
            },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Inline(
            objectiveai::functions::InlineProfile::Auto(ten_tweet_swarm_two_agents(
                Some(6),
                None,
                objectiveai::agent::mock::OutputMode::Instruction,
                objectiveai::agent::mock::OutputMode::Instruction,
                vec![Decimal::ONE, Decimal::ONE],
            )),
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input,
        split: Some(true),
        invert: None,
        provider: None,
        seed: Some(42),
        stream: Some(true),
        continuation: None,
    };
    let result = normalize(run_execution_split(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/split_tweet_scorer_10_tweets_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/split_tweet_scorer_10_tweets_seed_42.json"),
    );
}

#[tokio::test]
async fn test_vector_tweet_ranker_10_tweets_seed_42() {
    let items: InputValue = serde_json::from_str(include_str!(
        "../../assets/functions/executions/client_tests/inputs/10_tweets.json"
    )).expect("10_tweets.json must parse as InputValue");
    let request = FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: "tweet-ranker".to_string(),
            },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Inline(
            objectiveai::functions::InlineProfile::Auto(ten_tweet_swarm_two_agents(
                None,
                Some(3),
                objectiveai::agent::mock::OutputMode::ToolCall,
                objectiveai::agent::mock::OutputMode::JsonSchema,
                vec![Decimal::new(4, 1), Decimal::new(6, 1)],
            )),
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input: InputValue::Object(indexmap::indexmap! {
            "items".into() => items,
        }),
        split: None,
        invert: None,
        provider: None,
        seed: Some(42),
        stream: Some(true),
        continuation: None,
    };
    let result = normalize(run_execution(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/vector_tweet_ranker_10_tweets_seed_42.json"),
        include_str!("../../assets/functions/executions/client_tests/vector_tweet_ranker_10_tweets_seed_42.json"),
    );
}
