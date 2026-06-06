//! Recursive-invention snapshot suite driven through the SDK
//! `BinaryExecutor`. Each test builds a typed
//! `functions::inventions::recursive::create::remote::Request`,
//! streams `ResponseItem` chunks, accumulates them into a
//! `FunctionInventionRecursive`, normalises it via the Rust SDK's
//! `normalize_for_tests`, and structurally compares the **whole**
//! rounded result against the canonical api-side snapshot at
//! `objectiveai-api/assets/functions/inventions/recursive_client_tests/`.
//!
//! Mirrors the canonical 3-SDK pattern in
//! `objectiveai-sdk-py/tests/http_test_util.py`,
//! `objectiveai-sdk-js/src/httpTestUtil.ts`, and
//! `objectiveai-sdk-go/tests/http_test_util_test.go`. The cli stays
//! streaming-only by transport-level necessity; every other piece of
//! the canonical pattern carries over verbatim.

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

fn snapshot_path(name: &str) -> PathBuf {
    snapshots_dir().join(format!("{name}.json"))
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
    cli_test_util::assert_normalized_snapshot(
        &snapshot_path("valid_schema_valid_tasks_scalar_leaf"),
        "valid_schema_valid_tasks_scalar_leaf",
        &result,
    );
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
    cli_test_util::assert_normalized_snapshot(
        &snapshot_path("valid_vector_schema_valid_tasks"),
        "valid_vector_schema_valid_tasks",
        &result,
    );
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
    cli_test_util::assert_normalized_snapshot(
        &snapshot_path("valid_schema_no_tasks_with_essay"),
        "valid_schema_no_tasks_with_essay",
        &result,
    );
}
