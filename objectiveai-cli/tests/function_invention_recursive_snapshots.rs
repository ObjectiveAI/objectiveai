mod cli_test_util;

use std::path::{Path, PathBuf};

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

/// Assert CLI invention output matches snapshot expectations.
fn assert_invention_snapshot(snapshot_name: &str, cli_result: &serde_json::Value) {
    let snapshot = cli_test_util::load_snapshot(&snapshots_dir(), snapshot_name);
    let expected_names = snapshot_invention_names(&snapshot);
    let has_errors = snapshot_has_errors(&snapshot);

    let results = cli_result.as_array()
        .expect("CLI output should be an array");

    assert_eq!(
        results.len(), expected_names.len(),
        "invention count mismatch for {}: got {} expected {}",
        snapshot_name, results.len(), expected_names.len()
    );

    let actual_names: Vec<String> = results.iter()
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(actual_names, expected_names, "invention names mismatch for {}", snapshot_name);

    if has_errors {
        assert!(
            results.iter().any(|r| r.get("error").is_some_and(|e| !e.is_null())),
            "expected errors for {} but got none", snapshot_name
        );
    }
}

// ---------------------------------------------------------------------------
// Remote mock state tests — identical to objectiveai-js and objectiveai-py
// ---------------------------------------------------------------------------

/// Scalar leaf with valid input schema, tasks, and description.
/// Matches JS/Python: valid_schema_valid_tasks_scalar_leaf, seed 5300.
#[test]
fn valid_schema_valid_tasks_scalar_leaf() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping valid_schema_valid_tasks_scalar_leaf");
        return;
    }
    let result = cli_test_util::run_cli(&[
        "functions", "inventions", "recursive", "create", "remote",
        "--state", "remote=mock,name=inv-good-sl",
        "--agent", "remote=mock,name=invention",
        "--seed", "5300",
    ]);
    assert_invention_snapshot("valid_schema_valid_tasks_scalar_leaf", &result);
}

/// Vector leaf with valid input schema and tasks.
/// Matches JS/Python: valid_vector_schema_valid_tasks, seed 5400.
#[test]
fn valid_vector_schema_valid_tasks() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping valid_vector_schema_valid_tasks");
        return;
    }
    let result = cli_test_util::run_cli(&[
        "functions", "inventions", "recursive", "create", "remote",
        "--state", "remote=mock,name=inv-good-vl",
        "--agent", "remote=mock,name=invention",
        "--seed", "5400",
    ]);
    assert_invention_snapshot("valid_vector_schema_valid_tasks", &result);
}

/// Scalar leaf with valid input schema and essay but no tasks.
/// Matches JS/Python: valid_schema_no_tasks_with_essay, seed 5900.
#[test]
fn valid_schema_no_tasks_with_essay() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping valid_schema_no_tasks_with_essay");
        return;
    }
    let result = cli_test_util::run_cli(&[
        "functions", "inventions", "recursive", "create", "remote",
        "--state", "remote=mock,name=inv-schema-only",
        "--agent", "remote=mock,name=invention",
        "--seed", "5900",
    ]);
    assert_invention_snapshot("valid_schema_no_tasks_with_essay", &result);
}
