//! End-to-end: a two-agent mock swarm runs ONE vector-completion
//! task inside a function execution; each agent uses the same plugin
//! fixture (`test-mcp-plugin-foo-headers`) but with a different `foo`
//! argument (`"A"` vs `"B"`). The plugin echoes `foo` as its
//! `Mcp-Session-Id` on initialize and writes one line to
//! `<OBJECTIVEAI_STATE_DIR>/<foo>.txt` per `invoke` tool call, where `OBJECTIVEAI_STATE_DIR`
//! is the plugin's per-state scratch dir
//! `<dir>/state/<test>/plugins/testorg/test-mcp-plugin-foo-headers/1.0.0`.
//!
//! Each agent's `calls` override emits two scripted turns: turn 1
//! calls the `invoke` tool, turn 2 closes out with content. After the
//! run, the test asserts `A.txt` / `B.txt` were created with the
//! expected line — proving the per-agent `X-OBJECTIVEAI-ARGUMENTS`
//! map round-trips through API → CLI conduit → plugin argv, and that
//! `Mcp-Session-Id` routes calls back to the matching plugin
//! instance.

mod cli_test_util;

use objectiveai_sdk::cli::command::functions::execute::standard::{
    Request, RequestDangerousAdvanced, RequestInput, ResponseItem,
};
use objectiveai_sdk::cli::command::functions::execute::{
    FunctionSpec, ProfileSpec,
};
use objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional;
use objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional;
use serde_json::json;

/// Inline mock-agent JSON: drives the deterministic `calls` script
/// (turn 1: `invoke`, turn 2: content) and points the agent at the
/// fixture's `demo` MCP via `client_objectiveai_mcp.plugins`, passing
/// `foo: <foo_value>` as the plugin argument so each agent gets its
/// own dedicated plugin process + scratch file.
///
/// The tool name `test-foo-plugin_invoke` is composed of the
/// upstream's `serverInfo.name` (`test-foo-plugin`) plus the tool's
/// own name (`invoke`); that's how `objectiveai-mcp-proxy` prefixes
/// every advertised tool — see `session::prefix_name`.
fn mock_agent(foo_value: &str) -> serde_json::Value {
    json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "client_objectiveai_mcp": {
            "plugins": [{
                "owner": "testorg",
                "name": "test-mcp-plugin-foo-headers",
                "version": "1.0.0",
                "executable": false,
                "mcp_servers": [{
                    "name": "demo",
                    "arguments": { "foo": foo_value }
                }]
            }]
        },
        "calls": [
            {
                "tool_calls": [
                    { "name": "test-foo-plugin_invoke", "arguments": "{}" }
                ],
                "content": ""
            },
            {
                "tool_calls": [],
                "content": format!("done-{foo_value}")
            }
        ]
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn function_swarm_writes_per_agent_files() {
    let base = cli_test_util::test_base_dir();

    // One-task vector function. `output: {"$special":"output"}`
    // passes the task result through unchanged — the test asserts on
    // filesystem side-effects, not on the function's score vector.
    // Special variants are serde-renamed to snake_case.
    let function_json = json!({
        "type": "vector.function",
        "tasks": [{
            "type": "vector.completion",
            "messages": [{ "role": "user", "content": "pick one" }],
            "responses": ["alpha", "beta"],
            "output": { "$special": "output" }
        }]
    });

    // Two-agent inline swarm. Profile is `InlineProfile::Auto`
    // (untagged: a swarm-shaped JSON lands there directly) which
    // applies the same swarm + uniform weights to every vector
    // completion task in the function — exactly one task here.
    let profile_json = json!({
        "agents": [mock_agent("A"), mock_agent("B")],
        "weights": [1.0, 1.0]
    });

    let function = FunctionSpec::Resolved(
        serde_json::from_value::<FullInlineFunctionOrRemoteCommitOptional>(function_json)
            .expect("function JSON must deserialize"),
    );
    let profile = ProfileSpec::Resolved(
        serde_json::from_value::<InlineProfileOrRemoteCommitOptional>(profile_json)
            .expect("profile JSON must deserialize"),
    );

    let request = Request { path_type: objectiveai_sdk::cli::command::functions::execute::standard::Path::FunctionsExecuteStandard,
        function,
        profile,
        input: RequestInput::Inline(
            serde_json::from_value(json!({})).expect("empty input deserializes"),
        ),
        continuation: None,
        split: false,
        invert: false,
        // Stream so the cli waits for the function execution to fully
        // finish before exiting. Without it the cli emits a bare `Id`
        // and detaches from the instance subprocess on `LogStreamReady`,
        // leaving the instance to write `A.txt`/`B.txt` orphaned —
        // the assertions below would race against those writes.
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(42),
        }),
        base: Default::default(),
    };

    let executor = cli_test_util::executor().await;
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    assert!(
        !items.is_empty(),
        "function executor must emit at least one chunk"
    );

    // Each agent calls `invoke` once during turn 1 of its scripted
    // `calls` override (writing one `<foo> - <foo>` line to its own
    // file), then — since a mock never emits a valid vote key — the
    // vector-completion client retries and the mock falls through to
    // its RNG-driven dispatcher (`resolve_mock_response`), which may
    // emit any number of additional parallel `invoke` calls.
    //
    // That extra count is a deterministic-but-incidental function of
    // the mock's per-turn RNG seed, which derives from
    // `prompt::id(messages)` — so it shifts whenever message
    // serialization changes at all (it is the same reason the snapshot
    // suites move). We therefore assert the ROUTING invariant this test
    // exists to prove, not a brittle exact line count: each file exists,
    // is non-empty (the scripted call landed), and EVERY line is
    // `<foo> - <foo>` — i.e. agent A's calls only ever reached `A.txt`
    // and agent B's only `B.txt`, proving the per-agent `foo` argv +
    // `Mcp-Session-Id` routed every call to the matching plugin process.
    let plugin_state_dir = base
        .join("plugins")
        .join("testorg")
        .join("test-mcp-plugin-foo-headers")
        .join("1.0.0");
    for foo in ["A", "B"] {
        let path = plugin_state_dir.join(format!("{foo}.txt"));
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("missing {}: {e}", path.display()));
        assert!(
            !content.is_empty(),
            "{foo}.txt is empty — the scripted invoke never landed"
        );
        let expected_line = format!("{foo} - {foo}");
        for line in content.lines() {
            assert_eq!(
                line, expected_line,
                "{foo}.txt has a misrouted line: {line:?} (whole file: {content:?})"
            );
        }
    }
}
