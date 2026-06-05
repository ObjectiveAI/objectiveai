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

/// Run a recursive-invention create through the executor and return
/// the chunks as plain JSON values so the existing snapshot diffs
/// don't have to know about the typed shapes.
async fn run_remote(state_name: &str, seed: i64) -> Vec<serde_json::Value> {
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
        // has chunks to filter into the returned JSON list.
        dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
        jq: None,
    };
    let executor = cli_test_util::executor();
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    items
        .into_iter()
        .filter_map(|item| match item {
            ResponseItem::Chunk(chunk) => serde_json::to_value(chunk).ok(),
            ResponseItem::Id(_) => None,
        })
        .collect()
}

/// Assert CLI invention output matches snapshot expectations.
fn assert_invention_snapshot(snapshot_name: &str, results: &[serde_json::Value]) {
    let snapshot = cli_test_util::load_snapshot(&snapshots_dir(), snapshot_name);
    let expected_names = snapshot_invention_names(&snapshot);
    let has_errors = snapshot_has_errors(&snapshot);

    assert_eq!(
        results.len(),
        expected_names.len(),
        "invention count mismatch for {}: got {} expected {}",
        snapshot_name,
        results.len(),
        expected_names.len()
    );

    let actual_names: Vec<String> = results
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        actual_names, expected_names,
        "invention names mismatch for {}",
        snapshot_name
    );

    if has_errors {
        assert!(
            results
                .iter()
                .any(|r| r.get("error").is_some_and(|e| !e.is_null())),
            "expected errors for {} but got none",
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
    let results = run_remote("inv-good-sl", 5300).await;
    assert_invention_snapshot("valid_schema_valid_tasks_scalar_leaf", &results);
}

/// Vector leaf with valid input schema and tasks.
/// Matches JS/Python: valid_vector_schema_valid_tasks, seed 5400.
#[tokio::test]
async fn valid_vector_schema_valid_tasks() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping valid_vector_schema_valid_tasks");
        return;
    }
    let results = run_remote("inv-good-vl", 5400).await;
    assert_invention_snapshot("valid_vector_schema_valid_tasks", &results);
}

/// Scalar leaf with valid input schema and essay but no tasks.
/// Matches JS/Python: valid_schema_no_tasks_with_essay, seed 5900.
#[tokio::test]
async fn valid_schema_no_tasks_with_essay() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping valid_schema_no_tasks_with_essay");
        return;
    }
    let results = run_remote("inv-schema-only", 5900).await;
    assert_invention_snapshot("valid_schema_no_tasks_with_essay", &results);
}
