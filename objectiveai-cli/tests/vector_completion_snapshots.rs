//! Snapshot tests for the CLI driving `vector completions post`.
//!
//! Sibling to `agent_completion_snapshots.rs`. Snapshots live under
//! `objectiveai-cli/assets/vector/completions/snapshots/` (CLI-side
//! assets — the API-side `client_tests/` directory is for API
//! integration test snapshots).
//!
//! Currently exercises one scenario: a 20-agent mock swarm in
//! JsonSchema output mode where every agent declares 10 entries in
//! `client_objectiveai_mcp.tools` (no plugins, `objectiveai` field
//! omitted). Driven through the CLI's `objectiveai api vector
//! completions post --body-inline …` command. Streaming requests
//! always go over WebSocket so the API can synthesize the per-agent
//! `client_objectiveai_mcp` reverse-attach URLs and bridge MCP
//! proxy traffic back to the CLI's `ConduitMcpHandler`.
//! Deserializes each emitted JSONL chunk as a `VectorCompletionChunk`,
//! aggregates via the SDK's `push` method, converts to the unary
//! `VectorCompletion` shape, and `normalize_for_tests`-es to zero
//! `id` / `created` and sort completions+votes for determinism.
//!
//! Set `UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS=1` to (re)write
//! the snapshot, matching the API integration suite's convention.

mod cli_test_util;

use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;
use objectiveai_sdk::vector::completions::response::unary::VectorCompletion;
use std::path::{Path, PathBuf};

fn snapshots_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/vector/completions/snapshots")
}

fn assert_snapshot(actual: &str, name: &str) {
    let path = snapshots_dir().join(format!("{name}.json"));
    if std::env::var("UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS").as_deref()
        == Ok("1")
    {
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

#[test]
fn test_twenty_agents_json_schema_10x_tools_seed_42() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "OBJECTIVEAI_TEST_PORT not set — skipping test_twenty_agents_json_schema_10x_tools_seed_42"
        );
        return;
    }

    let body = serde_json::json!({
        "messages": [{"role": "user", "content": "choose A or B"}],
        "responses": ["A", "B"],
        "swarm": {"remote": "mock", "name": "twenty-agents-json-schema-10x-tools"},
        "seed": 42,
        // `--ws` overrides this anyway, but kept for clarity since
        // the WS path is conceptually streaming.
        "stream": true,
    })
    .to_string();

    let chunks_json = cli_test_util::run_cli(&[
        "api", "vector", "completions", "post",
        "--body-inline", &body,
    ]);

    // Deserialize every emitted notification as a typed chunk.
    let chunks: Vec<VectorCompletionChunk> = chunks_json
        .as_array()
        .expect("CLI returned a single value, expected array of chunks")
        .iter()
        .map(|c| {
            serde_json::from_value(c.clone())
                .expect("failed to deserialize VectorCompletionChunk from CLI output")
        })
        .collect();
    assert!(!chunks.is_empty(), "no chunks emitted");

    // Aggregate: start from the first chunk, push the rest in.
    let mut chunks_iter = chunks.into_iter();
    let mut agg = chunks_iter.next().expect("at least one chunk");
    for chunk in chunks_iter {
        agg.push(&chunk);
    }

    // Convert to the unary shape and normalize for determinism.
    let mut result: VectorCompletion = agg.into();
    result.normalize_for_tests();

    let actual_str = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(&actual_str, "twenty_agents_json_schema_10x_tools_seed_42");
}
