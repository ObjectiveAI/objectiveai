//! Function-execution snapshot suite driven through the SDK
//! `BinaryExecutor`. Each macro invocation builds a typed
//! `functions::executions::create::standard::Request`, streams the cli
//! output, accumulates the `Chunk` variants into a single
//! `FunctionExecution`, and diffs the unary `.output` against the
//! api-side snapshot under
//! `objectiveai-api/assets/functions/executions/client_tests/`.

mod cli_test_util;

use std::path::{Path, PathBuf};

use objectiveai_sdk::RemotePathCommitOptional;
use objectiveai_sdk::cli::command::functions::executions::create::standard::{
    Request, RequestDangerousAdvanced, RequestInput, ResponseItem,
};
use objectiveai_sdk::cli::command::functions::executions::create::{
    FunctionSpec, ProfileSpec,
};
use objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional;
use objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
use objectiveai_sdk::functions::executions::response::unary::FunctionExecution;

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

/// Build the `function`/`profile` specs from mock fixture names.
fn mock_function_spec(name: &str) -> FunctionSpec {
    FunctionSpec::Resolved(FullInlineFunctionOrRemoteCommitOptional::Remote(
        RemotePathCommitOptional::Mock {
            name: name.to_string(),
        },
    ))
}

fn mock_profile_spec(name: &str) -> ProfileSpec {
    ProfileSpec::Resolved(InlineProfileOrRemoteCommitOptional::Remote(
        RemotePathCommitOptional::Mock {
            name: name.to_string(),
        },
    ))
}

fn inline_profile_spec(json: serde_json::Value) -> ProfileSpec {
    ProfileSpec::Resolved(
        serde_json::from_value::<InlineProfileOrRemoteCommitOptional>(json)
            .expect("inline profile JSON must deserialize"),
    )
}

/// Drive the executor, collect every `Chunk`, push them into a single
/// `FunctionExecutionChunk`, then convert to the unary form.
async fn run_and_aggregate(request: Request) -> FunctionExecution {
    let executor = cli_test_util::executor();
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    let mut chunks = items.into_iter().filter_map(|item| match item {
        ResponseItem::Chunk(c) => Some(c),
        ResponseItem::Id(_) => None,
    });
    let mut agg: FunctionExecutionChunk =
        chunks.next().expect("at least one chunk must be emitted");
    for chunk in chunks {
        agg.push(&chunk);
    }
    agg.into()
}

/// Assert the executed function's `.output` matches the snapshot's
/// `output.output`. Mirrors the legacy helper that used
/// `cli_result["output"]`.
fn assert_output_matches(snapshot_name: &str, execution: &FunctionExecution) {
    let snapshot = cli_test_util::load_snapshot(&snapshots_dir(), snapshot_name);
    let expected_output = cli_test_util::rounded(&snapshot_output(&snapshot));
    let has_errors = snapshot_has_errors(&snapshot);

    let execution_json = serde_json::to_value(execution).expect("FunctionExecution serializes");
    let actual_output = cli_test_util::rounded(&execution_json["output"]);
    assert_eq!(
        actual_output, expected_output,
        "output mismatch for {snapshot_name}",
    );

    if has_errors {
        assert!(
            execution_json
                .get("errors")
                .is_some_and(|e| e.as_array().is_some_and(|a| !a.is_empty())),
            "expected errors for {snapshot_name} but got none",
        );
    } else {
        assert!(
            execution_json.get("errors").is_none()
                || execution_json["errors"]
                    .as_array()
                    .is_some_and(|a| a.is_empty()),
            "expected no errors for {snapshot_name} but got: {:?}",
            execution_json.get("errors"),
        );
    }
}

macro_rules! snapshot_test {
    ($name:ident, $snapshot:expr, $function:expr, $profile:expr, $seed:expr, $input:tt) => {
        #[tokio::test]
        async fn $name() {
            if cli_test_util::test_api_address().is_none() {
                eprintln!(
                    "OBJECTIVEAI_TEST_PORT not set — skipping {}",
                    stringify!($name)
                );
                return;
            }
            let request = Request {
                path_type: objectiveai_sdk::cli::command::functions::executions::create::standard::Path::FunctionsExecutionsCreateStandard,
                function: mock_function_spec($function),
                profile: mock_profile_spec($profile),
                input: RequestInput::Inline(
                    serde_json::from_value(serde_json::json!($input))
                        .expect("input must deserialize as InputValue"),
                ),
                continuation: None,
                retry_token: None,
                seed: Some($seed),
                split: false,
                invert: false,
                // Stream so collect_stream's `ResponseItem::Chunk(_)`
                // loop has chunks to consume; without it the cli
                // emits only a bare `Id` and the aggregator is empty.
                dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
                jq: None,
            };
            let execution = run_and_aggregate(request).await;
            assert_output_matches($snapshot, &execution);
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
#[tokio::test]
async fn split_tweet_scorer_10_tweets_seed_42() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!("OBJECTIVEAI_TEST_PORT not set — skipping split_tweet_scorer_10_tweets_seed_42");
        return;
    }
    let snapshots = snapshots_dir();
    let input_path = snapshots.join("inputs/10_tweets.json");
    let input_value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&input_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", input_path.display())),
    )
    .expect("input JSON must parse");

    let profile_json = serde_json::json!({
        "agents": [
            {"count": 1, "upstream": "mock", "output_mode": "instruction", "top_logprobs": 6},
            {"count": 1, "upstream": "mock", "output_mode": "instruction"}
        ],
        "weights": [1.0, 1.0]
    });

    let request = Request {
        path_type: objectiveai_sdk::cli::command::functions::executions::create::standard::Path::FunctionsExecutionsCreateStandard,
        function: mock_function_spec("tweet-scorer"),
        profile: inline_profile_spec(profile_json),
        input: RequestInput::Inline(
            serde_json::from_value(input_value)
                .expect("tweets input must deserialize as InputValue"),
        ),
        continuation: None,
        retry_token: None,
        seed: Some(42),
        split: true,
        invert: false,
        // Stream so collect_stream's `ResponseItem::Chunk(_)` loop has
        // chunks to consume; without it the cli emits only a bare `Id`
        // and the aggregator is empty.
        dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
        jq: None,
    };

    let execution = run_and_aggregate(request).await;
    assert_output_matches("split_tweet_scorer_10_tweets_seed_42", &execution);
}
