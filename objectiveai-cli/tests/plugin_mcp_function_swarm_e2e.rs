//! End-to-end: a two-agent mock swarm runs ONE vector-completion
//! task inside a function execution; each agent uses the same plugin
//! fixture (`test-mcp-plugin-foo-headers`) but with a different `foo`
//! argument (`"A"` vs `"B"`). The plugin echoes `foo` as its
//! `Mcp-Session-Id` on initialize and writes one line to
//! `<CONFIG_BASE_DIR>/<foo>.txt` per `invoke` tool call.
//!
//! Each agent's `calls` override emits two scripted turns: turn 1
//! calls the `invoke` tool, turn 2 closes out with content. After the
//! run, the test asserts `A.txt` / `B.txt` were created with the
//! expected line — proving the per-agent `X-OBJECTIVEAI-ARGUMENTS`
//! map round-trips through API → CLI conduit → plugin argv, and that
//! `Mcp-Session-Id` routes calls back to the matching plugin
//! instance.
//!
//! Skip-gate: `OBJECTIVEAI_TEST_PORT` must point at a running test
//! API (same gate as every other cli e2e test).

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
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "skipping function_swarm_writes_per_agent_files: OBJECTIVEAI_TEST_PORT not set"
        );
        return;
    }

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
        retry_token: None,
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
        jq: None,
    };

    let executor = cli_test_util::executor().await;
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    assert!(
        !items.is_empty(),
        "function executor must emit at least one chunk"
    );

    // Both agents called `invoke` once during turn 1 of their
    // scripted `calls` override. After the script is exhausted, the
    // vector-completion client sends a continuation; the mock then
    // falls through to its RNG-driven dispatcher (~75% tool call,
    // ~25% respond-as-is per `resolve_mock_response`). With test
    // seed=42 and these specific per-agent continuation prompts, the
    // mock's RNG deterministically rolls:
    //   - agent A's continuation → respond-as-is (no extra invoke)
    //   - agent B's continuation → invoke (+1 extra invoke)
    // so `A.txt` ends up with one line and `B.txt` with two. The
    // plugin's `Mcp-Session-Id` assert ensures each call landed on
    // the matching plugin process; finding the file at all proves the
    // per-agent argv arrived correctly.
    let a_path = base.join("A.txt");
    let b_path = base.join("B.txt");
    let a = std::fs::read_to_string(&a_path)
        .unwrap_or_else(|e| panic!("missing {}: {e}", a_path.display()));
    let b = std::fs::read_to_string(&b_path)
        .unwrap_or_else(|e| panic!("missing {}: {e}", b_path.display()));
    assert_eq!(a, "A - A\n");
    assert_eq!(b, "B - B\nB - B\n");
}
