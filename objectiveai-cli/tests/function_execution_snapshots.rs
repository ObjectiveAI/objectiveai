//! Function-execution snapshot suite driven through the SDK
//! `BinaryExecutor`. Each macro invocation builds a typed
//! `functions::executions::create::standard::Request`, streams the cli
//! output, accumulates the `Chunk` variants into a single
//! `FunctionExecution`, normalises it via the Rust SDK's
//! `normalize_for_tests`, and structurally compares the **whole**
//! rounded `FunctionExecution` against the canonical api-side
//! snapshot at
//! `objectiveai-api/assets/functions/executions/client_tests/`.
//!
//! Mirrors the canonical 3-SDK pattern in
//! `objectiveai-sdk-py/tests/http_test_util.py`,
//! `objectiveai-sdk-js/src/httpTestUtil.ts`, and
//! `objectiveai-sdk-go/tests/http_test_util_test.go`. The cli stays
//! streaming-only by transport-level necessity (the cli's
//! `dangerous_advanced.stream = false` returns just an `Id`, not a
//! unary `FunctionExecution`); every other piece of the canonical
//! pattern carries over verbatim.

mod cli_test_util;

use std::path::{Path, PathBuf};

use objectiveai_sdk::RemotePathCommitOptional;
use objectiveai_sdk::cli::command::functions::execute::standard::{
    Request, RequestDangerousAdvanced, RequestInput, ResponseItem,
};
use objectiveai_sdk::cli::command::functions::execute::{
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
/// `FunctionExecutionChunk`, convert to the unary form, and apply
/// the Rust SDK's `normalize_for_tests` so the result is ready for
/// the structural-Value compare in [`assert_normalized_snapshot`].
async fn run_and_aggregate_normalized(request: Request) -> FunctionExecution {
    let executor = cli_test_util::executor().await;
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
    let mut execution: FunctionExecution = agg.into();
    execution.normalize_for_tests();
    execution
}

macro_rules! snapshot_test {
    ($name:ident, $snapshot:expr, $function:expr, $profile:expr, $seed:expr, $input:tt) => {
        #[tokio::test]
        async fn $name() {
            let request = Request {
                path_type: objectiveai_sdk::cli::command::functions::execute::standard::Path::FunctionsExecuteStandard,
                function: mock_function_spec($function),
                profile: mock_profile_spec($profile),
                input: RequestInput::Inline(
                    serde_json::from_value(serde_json::json!($input))
                        .expect("input must deserialize as InputValue"),
                ),
                continuation: None,
                retry_token: None,
                split: false,
                invert: false,
                // Stream so collect_stream's `ResponseItem::Chunk(_)`
                // loop has chunks to consume; without it the cli
                // emits only a bare `Id` and the aggregator is empty.
                dangerous_advanced: Some(RequestDangerousAdvanced {
                    stream: Some(true),
                    seed: Some($seed),
                }),
                base: Default::default(),
            };
            let execution = run_and_aggregate_normalized(request).await;
            let snapshot_path = snapshots_dir().join(format!("{}.json", $snapshot));
            cli_test_util::assert_normalized_snapshot(
                &snapshot_path,
                $snapshot,
                &execution,
            );
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
        path_type: objectiveai_sdk::cli::command::functions::execute::standard::Path::FunctionsExecuteStandard,
        function: mock_function_spec("tweet-scorer"),
        profile: inline_profile_spec(profile_json),
        input: RequestInput::Inline(
            serde_json::from_value(input_value)
                .expect("tweets input must deserialize as InputValue"),
        ),
        continuation: None,
        retry_token: None,
        split: true,
        invert: false,
        // Stream so collect_stream's `ResponseItem::Chunk(_)` loop has
        // chunks to consume; without it the cli emits only a bare `Id`
        // and the aggregator is empty.
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(42),
        }),
        base: Default::default(),
    };

    let execution = run_and_aggregate_normalized(request).await;
    let snapshot_path =
        snapshots_dir().join("split_tweet_scorer_10_tweets_seed_42.json");
    cli_test_util::assert_normalized_snapshot(
        &snapshot_path,
        "split_tweet_scorer_10_tweets_seed_42",
        &execution,
    );
}
