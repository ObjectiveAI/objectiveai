//! Recursive-invention snapshot suite driven through the SDK
//! `BinaryExecutor` rather than hand-rolled argv. Each test builds a
//! typed `functions::inventions::recursive::create::remote::Request`,
//! streams `ResponseItem` chunks back via the executor, and asserts
//! the resulting invention list against the api-side fixture under
//! `objectiveai-api/assets/functions/inventions/recursive_client_tests/`.

mod cli_test_util;

use std::path::{Path, PathBuf};

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::RemotePathCommitOptional;
use objectiveai_sdk::cli::command::agents::spawn::AgentSpec;
use objectiveai_sdk::cli::command::functions::inventions::recursive::create::remote::{
    Request, RequestDangerousAdvanced, RequestState, ResponseItem,
};
use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk;
use objectiveai_sdk::functions::inventions::recursive::response::unary::FunctionInventionRecursive;

fn snapshots_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../objectiveai-api/assets/functions/inventions/recursive_client_tests")
}

/// Extract expected invention names from a snapshot's inventions array.
fn snapshot_invention_names(snapshot: &serde_json::Value) -> Vec<String> {
    snapshot["inventions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|inv| {
            inv.get("state")
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
        })
        .collect()
}

/// Extract whether any inventions have errors in the snapshot.
fn snapshot_has_errors(snapshot: &serde_json::Value) -> bool {
    snapshot["inventions_errors"].as_bool().unwrap_or(false)
}

/// Run a recursive-invention create through the executor, aggregate
/// the streamed chunks into a single FunctionInventionRecursiveChunk,
/// convert to the unary FunctionInventionRecursive, and normalize for
/// test comparison. The cli emits ResponseItem::Chunk variants as
/// progressive updates to the same inventions, so counting chunks is
/// not the same as counting inventions — we have to push them through
/// the aggregator first.
async fn run_remote(state_name: &str, seed: i64) -> FunctionInventionRecursive {
    let request = Request { path_type: objectiveai_sdk::cli::command::functions::inventions::recursive::create::remote::Path::FunctionsInventionsRecursiveCreateRemote,
        state: RequestState::Ref(format!("remote=mock,name={state_name}")),
        agent: AgentSpec::Resolved(
            InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(
                RemotePathCommitOptional::Mock {
                    name: "invention".to_string(),
                },
            ),
        ),
        continuation: None,
        seed: Some(seed),
        // Stream so collect_stream's `ResponseItem::Chunk(_)` loop
        // has chunks to feed into the aggregator.
        dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
        jq: None,
    };
    let executor = cli_test_util::executor();
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    let mut chunks = items.into_iter().filter_map(|item| match item {
        ResponseItem::Chunk(c) => Some(c),
        ResponseItem::Id(_) => None,
    });
    let mut agg: FunctionInventionRecursiveChunk =
        chunks.next().expect("at least one chunk must be emitted");
    for chunk in chunks {
        agg.push(&chunk);
    }
    let mut unary: FunctionInventionRecursive = agg.into();
    unary.normalize_for_tests();
    unary
}

/// Assert CLI invention output matches snapshot expectations.
fn assert_invention_snapshot(snapshot_name: &str, result: &FunctionInventionRecursive) {
    let snapshot = cli_test_util::load_snapshot(&snapshots_dir(), snapshot_name);
    let expected_names = snapshot_invention_names(&snapshot);
    let has_errors = snapshot_has_errors(&snapshot);

    let result_json = serde_json::to_value(result).expect("FunctionInventionRecursive serializes");
    let inventions = result_json["inventions"].as_array().expect("inventions array");

    assert_eq!(
        inventions.len(),
        expected_names.len(),
        "invention count mismatch for {}: got {} expected {}",
        snapshot_name,
        inventions.len(),
        expected_names.len()
    );

    let actual_names: Vec<String> = inventions
        .iter()
        .map(|inv| {
            inv.get("state")
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();
    assert_eq!(
        actual_names, expected_names,
        "invention names mismatch for {}",
        snapshot_name
    );

    if has_errors {
        assert!(
            result.inventions_errors,
            "expected inventions_errors=true for {} but got false",
            snapshot_name
        );
    }
}

// ---------------------------------------------------------------------------
// Remote mock state tests — identical to objectiveai-js and objectiveai-py
// ---------------------------------------------------------------------------

/// Scalar leaf with valid input schema, tasks, and description.
/// Matches JS/Python: valid_schema_valid_tasks_scalar_leaf, seed 5300.
#[tokio::test]
async fn valid_schema_valid_tasks_scalar_leaf() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping valid_schema_valid_tasks_scalar_leaf");
        return;
    }
    let result = run_remote("inv-good-sl", 5300).await;
    assert_invention_snapshot("valid_schema_valid_tasks_scalar_leaf", &result);
}

/// Vector leaf with valid input schema and tasks.
/// Matches JS/Python: valid_vector_schema_valid_tasks, seed 5400.
#[tokio::test]
async fn valid_vector_schema_valid_tasks() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping valid_vector_schema_valid_tasks");
        return;
    }
    let result = run_remote("inv-good-vl", 5400).await;
    assert_invention_snapshot("valid_vector_schema_valid_tasks", &result);
}

/// Scalar leaf with valid input schema and essay but no tasks.
/// Matches JS/Python: valid_schema_no_tasks_with_essay, seed 5900.
#[tokio::test]
async fn valid_schema_no_tasks_with_essay() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping valid_schema_no_tasks_with_essay");
        return;
    }
    let result = run_remote("inv-schema-only", 5900).await;
    assert_invention_snapshot("valid_schema_no_tasks_with_essay", &result);
}
