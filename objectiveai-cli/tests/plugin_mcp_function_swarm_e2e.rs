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

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

use objectiveai_sdk::cli::command::functions::executions::create::standard::{
    Request, RequestInput, ResponseItem,
};
use objectiveai_sdk::cli::command::functions::executions::create::{
    FunctionSpec, ProfileSpec,
};
use objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional;
use objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional;
use serde_json::json;

static BUILD_PLUGIN_ONCE: Once = Once::new();

fn plugin_binary() -> PathBuf {
    let target_dir = cli_test_util::test_target_dir();
    let mut path = target_dir.join("debug/test-mcp-plugin-foo-headers");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    BUILD_PLUGIN_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args([
                "build",
                "-p",
                "test-mcp-plugin-foo-headers",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            .status()
            .expect("spawn cargo build test-mcp-plugin-foo-headers");
        assert!(status.success(), "test-mcp-plugin-foo-headers build failed");
    });
    path
}

/// Stage the fixture at the same layout
/// `objectiveai_cli::filesystem::Client::resolve_plugin` expects:
/// manifest at `<base>/plugins/<name>.json`, binary at
/// `<base>/plugins/<name>/plugin[.exe]`. Identical to the
/// `plugin_mcp_dispatch_e2e::stage_plugin` helper.
fn stage_plugin(base: &Path) {
    let plugins_dir = base.join("plugins");
    let install_dir = plugins_dir.join("test-mcp-plugin-foo-headers");
    std::fs::create_dir_all(&install_dir).unwrap();

    // Manifest. `url` is required by `Manifest::validate` but is not
    // read at runtime — `dial_plugin_upstream` always reads the live
    // URL off the plugin's stdout. A placeholder satisfies validation.
    let manifest = json!({
        "description": "foo-headers fixture",
        "version": "1.0.0",
        "owner": "testorg",
        "mcp_servers": [
            { "name": "demo", "url": "http://127.0.0.1:0", "authorization": false }
        ]
    });
    std::fs::write(
        plugins_dir.join("test-mcp-plugin-foo-headers.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let installed = install_dir.join(if cfg!(windows) {
        "plugin.exe"
    } else {
        "plugin"
    });
    std::fs::copy(plugin_binary(), &installed).expect("copy fixture binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&installed, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

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

    let _ = cli_test_util::cli_binary();
    let _ = plugin_binary();

    // Per-test base dir under `.objectiveai-tests/<binary>/<test>/`.
    // The plugin inherits `CONFIG_BASE_DIR` from the cli child, the
    // cli inherits it from the executor's `.env()`, and the per-
    // binary run-start clear handles wiping stale state — no manual
    // sub-cleanup needed.
    let base = cli_test_util::test_base_dir();

    stage_plugin(&base);

    // One-task vector function. `output: {"$special":"Output"}`
    // passes the task result through unchanged — the test asserts on
    // filesystem side-effects, not on the function's score vector.
    let function_json = json!({
        "type": "vector.function",
        "tasks": [{
            "type": "vector.completion",
            "messages": [{ "role": "user", "content": "pick one" }],
            "responses": ["alpha", "beta"],
            "output": { "$special": "Output" }
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

    let request = Request { path_type: objectiveai_sdk::cli::command::functions::executions::create::standard::Path::FunctionsExecutionsCreateStandard,
        function,
        profile,
        input: RequestInput::Inline(
            serde_json::from_value(json!({})).expect("empty input deserializes"),
        ),
        continuation: None,
        retry_token: None,
        seed: Some(42),
        split: false,
        invert: false,
        dangerous_advanced: None,
        jq: None,
    };

    let executor = cli_test_util::executor_with_base_dir(&base);
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    assert!(
        !items.is_empty(),
        "function executor must emit at least one chunk"
    );

    // Both agents called `invoke` once during turn 1 of their script.
    // The plugin's `Mcp-Session-Id` assert ensures each call landed
    // on the matching plugin process; finding the file at all is
    // what proves the per-agent argv arrived correctly.
    let a_path = base.join("A.txt");
    let b_path = base.join("B.txt");
    let a = std::fs::read_to_string(&a_path)
        .unwrap_or_else(|e| panic!("missing {}: {e}", a_path.display()));
    let b = std::fs::read_to_string(&b_path)
        .unwrap_or_else(|e| panic!("missing {}: {e}", b_path.display()));
    assert_eq!(a, "A - A\n");
    assert_eq!(b, "B - B\n");
}
