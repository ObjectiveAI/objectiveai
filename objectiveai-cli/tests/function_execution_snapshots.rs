mod cli_test_util;

use std::path::{Path, PathBuf};

fn snapshots_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../objectiveai-api/assets/functions/executions/client_tests")
}

fn snapshot_output(snapshot: &serde_json::Value) -> serde_json::Value {
    snapshot["output"]["output"].clone()
}

fn snapshot_has_errors(snapshot: &serde_json::Value) -> bool {
    snapshot["tasks_errors"].as_bool().unwrap_or(false)
}

macro_rules! snapshot_test {
    ($name:ident, $snapshot:expr, $function:expr, $profile:expr, $seed:expr, $input:tt) => {
        #[test]
        fn $name() {
            if cli_test_util::test_api_address().is_none() {
                eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping {}", stringify!($name));
                return;
            }
            let input = serde_json::to_string(&serde_json::json!($input)).unwrap();
            let seed_str = $seed.to_string();

            let function_str = format!("remote=mock,name={}", $function);
            let profile_str = format!("remote=mock,name={}", $profile);
            let instructions_id = cli_test_util::instructions_id(
                cli_test_util::InstructionsScope::FunctionExecutions,
            );
            let cli_result = cli_test_util::run_cli(&[
                "functions", "executions", "create", "standard",
                "--function", &function_str,
                "--profile", &profile_str,
                "--input-inline", &input,
                "--seed", &seed_str,
                "--instructions-id", instructions_id.as_str(),
            ]);

            let snapshot = cli_test_util::load_snapshot(&snapshots_dir(), $snapshot);
            let expected_output = cli_test_util::rounded(&snapshot_output(&snapshot));
            let has_errors = snapshot_has_errors(&snapshot);

            let actual_output = cli_test_util::rounded(&cli_result["output"]);
            assert_eq!(actual_output, expected_output, "output mismatch for {}", $snapshot);

            if has_errors {
                assert!(
                    cli_result.get("errors").is_some_and(|e| e.as_array().is_some_and(|a| !a.is_empty())),
                    "expected errors for {} but got none", $snapshot
                );
            } else {
                assert!(
                    cli_result.get("errors").is_none()
                        || cli_result["errors"].as_array().is_some_and(|a| a.is_empty()),
                    "expected no errors for {} but got: {:?}", $snapshot, cli_result.get("errors")
                );
            }
        }
    };
}

snapshot_test!(
    mock_1_scalar_leaf_binary_seed_42,
    "mock_1_scalar_leaf_binary_seed_42",
    "binary-classifier",
    "solo-instruction",
    42,
    {"text": "Hello world"}
);

snapshot_test!(
    mock_7_vector_5_criteria_seed_42,
    "mock_7_vector_5_criteria_seed_42",
    "five-criteria-ranker",
    "schema-heavy-trio",
    42,
    {"items": ["Option A", "Option B", "Option C"]}
);

snapshot_test!(
    mock_20_vector_super_branch_seed_42,
    "mock_20_vector_super_branch_seed_42",
    "nested-vector-super-branch",
    "nested-vector-inline-remote",
    42,
    {"items": ["Alpha", "Beta", "Gamma"]}
);

/// Split: tweet-scorer over 10 real tweets (input loaded from
/// `inputs/10_tweets.json`), seed 42. Mirrors the Rust api test
/// `test_split_tweet_scorer_10_tweets_seed_42`: same mock function,
/// same inline profile (two mock instruction agents, one with
/// top_logprobs=6, equal weights), same input file, same seed.
#[test]
fn split_tweet_scorer_10_tweets_seed_42() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping split_tweet_scorer_10_tweets_seed_42");
        return;
    }
    let snapshots = snapshots_dir();
    let input_path = snapshots.join("inputs/10_tweets.json");
    let input = std::fs::read_to_string(&input_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", input_path.display()));

    let profile_inline = r#"{
        "agents": [
          {"count": 1, "upstream": "mock", "output_mode": "instruction", "top_logprobs": 6},
          {"count": 1, "upstream": "mock", "output_mode": "instruction"}
        ],
        "weights": [1.0, 1.0]
    }"#;

    let id = cli_test_util::instructions_id(
        cli_test_util::InstructionsScope::FunctionExecutions,
    );
    let cli_result = cli_test_util::run_cli(&[
        "functions", "executions", "create", "standard",
        "--function", "remote=mock,name=tweet-scorer",
        "--profile-inline", profile_inline,
        "--input-inline", &input,
        "--split",
        "--seed", "42",
        "--instructions-id", id.as_str(),
    ]);

    let snapshot = cli_test_util::load_snapshot(&snapshots, "split_tweet_scorer_10_tweets_seed_42");
    let expected_output = cli_test_util::rounded(&snapshot_output(&snapshot));
    let has_errors = snapshot_has_errors(&snapshot);

    let actual_output = cli_test_util::rounded(&cli_result["output"]);
    assert_eq!(actual_output, expected_output, "output mismatch for split_tweet_scorer_10_tweets_seed_42");

    if has_errors {
        assert!(
            cli_result.get("errors").is_some_and(|e| e.as_array().is_some_and(|a| !a.is_empty())),
            "expected errors but got none",
        );
    } else {
        assert!(
            cli_result.get("errors").is_none()
                || cli_result["errors"].as_array().is_some_and(|a| a.is_empty()),
            "expected no errors but got: {:?}", cli_result.get("errors"),
        );
    }
}
