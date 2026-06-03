//! Snapshot test for a vector-output function execution driven through
//! the SDK `BinaryExecutor`.
//!
//! Originally `vector_completion_snapshots.rs` invoked the legacy
//! `api vector completions post` command directly. That command path
//! no longer exists in the new bare-naked tree, so the workload is
//! now wrapped one tier higher — a mock function whose only task is a
//! single `vector.completion` over a 20-agent JsonSchema swarm.
//!
//! Currently exercises one scenario: the same 20-agent mock swarm in
//! JsonSchema output mode where every agent declares 10 entries in
//! `client_objectiveai_mcp.tools` (no plugins, `objectiveai` field
//! omitted). The function execution accumulates `FunctionExecutionChunk`s
//! into a unary `FunctionExecution`, normalizes for determinism, and
//! diffs the serialized form against the CLI-side snapshot under
//! `objectiveai-cli/assets/vector/completions/snapshots/`.
//!
//! Set `UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS=1` to (re)write
//! the snapshot, matching the API integration suite's convention.

mod cli_test_util;

use std::path::{Path, PathBuf};

use objectiveai_sdk::RemotePathCommitOptional;
use objectiveai_sdk::cli::command::functions::executions::create::standard::{
    Request, RequestInput, ResponseItem,
};
use objectiveai_sdk::cli::command::functions::executions::create::{
    FunctionSpec, ProfileSpec,
};
use objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional;
use objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
use objectiveai_sdk::functions::executions::response::unary::FunctionExecution;

fn snapshots_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/vector/completions/snapshots")
}

fn assert_snapshot(actual: &str, name: &str) {
    let path = snapshots_dir().join(format!("{name}.json"));
    if std::env::var("UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS").as_deref() == Ok("1") {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, format!("{actual}\n")).unwrap();
        eprintln!("Updated snapshot: {}", path.display());
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read snapshot {}: {e}", path.display()));
    assert_eq!(
        actual,
        expected.trim_end(),
        "snapshot mismatch for {}",
        path.display(),
    );
}

#[tokio::test]
async fn test_twenty_agents_json_schema_10x_tools_seed_42() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "OBJECTIVEAI_TEST_PORT not set — skipping test_twenty_agents_json_schema_10x_tools_seed_42"
        );
        return;
    }

    // Mock function fixture whose body is a single `vector.completion`
    // task over the 20-agent JsonSchema swarm with the 10-tools surface
    // every agent declares. Lives on the api side; if the fixture
    // hasn't landed there yet, expect a NotFound error from the
    // executor.
    let function = FunctionSpec::Resolved(FullInlineFunctionOrRemoteCommitOptional::Remote(
        RemotePathCommitOptional::Mock {
            name: "twenty-agents-json-schema-10x-tools-vector".to_string(),
        },
    ));

    // Mock profile fixture supplying the per-agent weights for the
    // wrapper task. Same naming convention as the function.
    let profile = ProfileSpec::Resolved(InlineProfileOrRemoteCommitOptional::Remote(
        RemotePathCommitOptional::Mock {
            name: "twenty-agents-json-schema-10x-tools-profile".to_string(),
        },
    ));

    // Input mirrors the original `api vector completions post` body's
    // `messages` + `responses` fields. The mock function's task
    // expression unpacks these into the inner vector completion call.
    let input_json = serde_json::json!({
        "messages": [{"role": "user", "content": "choose A or B"}],
        "responses": ["A", "B"],
    });

    let request = Request {
        function,
        profile,
        input: RequestInput::Inline(
            serde_json::from_value(input_json).expect("input must deserialize as InputValue"),
        ),
        continuation: None,
        retry_token: None,
        seed: Some(42),
        split: false,
        invert: false,
        dangerous_advanced: None,
        jq: None,
    };

    let executor = cli_test_util::executor();
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    let mut chunks = items.into_iter().filter_map(|item| match item {
        ResponseItem::Chunk(c) => Some(c),
        ResponseItem::Id(_) => None,
    });
    let mut agg: FunctionExecutionChunk =
        chunks.next().expect("at least one function-execution chunk must be emitted");
    for chunk in chunks {
        agg.push(&chunk);
    }

    let mut result: FunctionExecution = agg.into();
    result.normalize_for_tests();

    let actual_str = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(&actual_str, "twenty_agents_json_schema_10x_tools_seed_42");
}
