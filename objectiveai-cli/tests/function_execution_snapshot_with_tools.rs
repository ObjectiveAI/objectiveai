//! Snapshot test for a function execution whose body is a single
//! `vector.completion` over a 20-agent JsonSchema swarm with a
//! 10-tools surface on every agent (no plugins, `objectiveai`
//! field omitted). Driven through the SDK `BinaryExecutor`.
//!
//! The function execution accumulates `FunctionExecutionChunk`s
//! into a unary `FunctionExecution`, normalises it via the Rust
//! SDK's `normalize_for_tests`, and structurally compares the
//! **whole** rounded result against the snapshot under
//! `objectiveai-cli/assets/function/executions/snapshots/`.
//!
//! Mirrors the canonical 3-SDK pattern in
//! `objectiveai-sdk-py/tests/http_test_util.py`,
//! `objectiveai-sdk-js/src/httpTestUtil.ts`, and
//! `objectiveai-sdk-go/tests/http_test_util_test.go`. The cli
//! stays streaming-only by transport-level necessity; every other
//! piece of the canonical pattern carries over verbatim.
//!
//! Set `UPDATE_SNAPSHOTS=1` to (re)write the snapshot — it
//! serialises the normalised rounded form so the committed file
//! matches what the assertion path reads back.

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
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/function/executions/snapshots")
}

/// Write `normalized` to the committed snapshot file in
/// `assets/function/executions/snapshots/` as a pretty-printed
/// rounded `Value`. Mirrors the read shape used by
/// [`cli_test_util::assert_normalized_snapshot`] so the committed
/// file round-trips byte-for-byte.
fn update_snapshot<T: serde::Serialize>(name: &str, normalized: &T) {
    let path = snapshots_dir().join(format!("{name}.json"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let value = serde_json::to_value(normalized).expect("normalized value serialises");
    let rounded_value = cli_test_util::rounded(&value);
    let pretty = serde_json::to_string_pretty(&rounded_value)
        .expect("rounded Value serialises to pretty JSON");
    std::fs::write(&path, format!("{pretty}\n")).unwrap();
    eprintln!("Updated snapshot: {}", path.display());
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

    // Input now matches the Alpha vector schema's required `items`
    // field — the mock function fixture hardcodes the prompt
    // messages and iterates `input['items']` for the per-response
    // votes (mirroring the item-ranker shape).
    let input_json = serde_json::json!({
        "items": ["A", "B"],
    });

    let request = Request { path_type: objectiveai_sdk::cli::command::functions::execute::standard::Path::FunctionsExecuteStandard,
        function,
        profile,
        input: RequestInput::Inline(
            serde_json::from_value(input_json).expect("input must deserialize as InputValue"),
        ),
        continuation: None,
        retry_token: None,
        split: false,
        invert: false,
        // Stream so the executor emits per-chunk `ResponseItem::Chunk(_)`
        // for the aggregator below; without it the cli emits only a
        // bare `Id` and the chunk loop is empty.
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(42),
        }),
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

    let name = "twenty_agents_json_schema_10x_tools_seed_42";
    if std::env::var("UPDATE_SNAPSHOTS").as_deref() == Ok("1") {
        update_snapshot(name, &result);
        return;
    }
    let snapshot_path = snapshots_dir().join(format!("{name}.json"));
    cli_test_util::assert_normalized_snapshot(&snapshot_path, name, &result);
}
