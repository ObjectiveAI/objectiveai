//! End-to-end snapshot: `agents spawn` against an inline mock agent
//! whose only MCP surface is a plugin-served RMCP upstream with 10
//! tools (`demo_tool0`..`demo_tool9`).
//!
//! Validates the CLI conduit's initialize-time plugin dial path:
//! the plugin binary is spawned by `dial_plugin_upstream`, the
//! conduit dials its RMCP server, the mock agent's tool surface
//! includes the 10 prefixed tools, and a tool call round-trips
//! through `tools/call` routing.
//!
//! The plugin RMCP server (a `test-mcp-plugin` binary) is killed
//! before the test returns via a `Drop` guard that reads the PID
//! the plugin wrote to `OAI_TEST_MCP_PID_FILE` before announcing
//! its URL. The entire `CONFIG_BASE_DIR` lives under the system
//! temp dir — nothing touches the host's `.objectiveai`.
//!
//! Driven through the SDK `BinaryExecutor` rather than hand-rolled
//! argv; the extra env var the plugin fixture needs is attached to
//! the executor via [`BinaryExecutor::env`].
//!
//! Skip-gate: requires `OBJECTIVEAI_TEST_PORT` to point at a
//! running test API (mirrors the existing snapshot tests).

mod cli_test_util;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;
use std::time::{Duration, Instant};

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::spawn::{
    AgentSpec, Request as SpawnRequest, RequestPrompt,
    ResponseItem as SpawnResponseItem,
};
use serde_json::{Value, json};

static BUILD_TEST_MCP_PLUGIN_ONCE: Once = Once::new();

fn test_mcp_plugin_binary() -> PathBuf {
    let target_dir = cli_test_util::test_target_dir();
    let mut path = target_dir.join("debug/test-mcp-plugin");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    BUILD_TEST_MCP_PLUGIN_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args([
                "build",
                "-p",
                "test-mcp-plugin",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            .status()
            .expect("spawn cargo build test-mcp-plugin");
        assert!(status.success(), "test-mcp-plugin build failed");
    });
    path
}

/// Per-test scratch dir under the system temp dir. Entirely
/// isolated — no contact with `~/.objectiveai` or the in-repo
/// `objectiveai-cli/tests/.objectiveai`.
fn temp_base() -> PathBuf {
    let d = std::env::temp_dir().join(format!("oai-mcp-plugin-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Stage the plugin: write its manifest to
/// `<base>/plugins/test-mcp-plugin.json` and copy the fixture binary
/// to `<base>/plugins/test-mcp-plugin/plugin[.exe]` — the layout
/// `objectiveai_sdk::filesystem::Client::resolve_plugin` expects.
fn stage_plugin(base: &Path) {
    let plugins_dir = base.join("plugins");
    let plugin_install_dir = plugins_dir.join("test-mcp-plugin");
    std::fs::create_dir_all(&plugin_install_dir).unwrap();

    // The `url` field is required by `Manifest::validate` but the
    // CLI's `dial_plugin_upstream` reads the actual URL from the
    // plugin's stdout — manifest URL is unused at runtime. A
    // placeholder keeps validate happy.
    let manifest = json!({
        "description": "test fixture",
        "version": "1.0.0",
        "owner": "testorg",
        "mcp_servers": [
            { "name": "demo", "url": "http://127.0.0.1:0", "authorization": false }
        ]
    });
    std::fs::write(
        plugins_dir.join("test-mcp-plugin.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let fixture = test_mcp_plugin_binary();
    let installed = plugin_install_dir.join(if cfg!(windows) {
        "plugin.exe"
    } else {
        "plugin"
    });
    std::fs::copy(&fixture, &installed).expect("copy fixture binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&installed, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// RAII cleanup: kill the plugin process (PID read from
/// `OAI_TEST_MCP_PID_FILE`) and remove the scratch dir. Runs on
/// success AND panic, so the plugin RMCP server never leaks past the
/// test boundary.
struct PluginGuard {
    pid_file: PathBuf,
    base: PathBuf,
}

impl Drop for PluginGuard {
    fn drop(&mut self) {
        if let Ok(s) = std::fs::read_to_string(&self.pid_file) {
            if let Ok(pid) = s.trim().parse::<u32>() {
                #[cfg(windows)]
                {
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string()])
                        .status();
                }
                #[cfg(unix)]
                {
                    let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
                }
            }
        }
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

/// Wait for the CLI-stream child to have flushed the agent's
/// response continuation and torn down its socket. Mirrors
/// `agents_continuation_tool_session_e2e::wait_for_completion`.
async fn wait_for_completion(base_dir: &Path, spawn_id: &str) {
    let response_cont_path = base_dir
        .join("logs/agents/completions/response/continuation")
        .join(format!("{spawn_id}.json"));
    let socket_path = base_dir.join("pipes/cli").join(spawn_id).join("socket");
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if response_cont_path.exists() && !socket_path.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cli-stream did not flush continuation + tear down socket for {spawn_id} in 180s",);
}

/// Extract a deterministic snapshot projection from the response
/// continuation JSON: every assistant tool call's `name`, and every
/// tool-result message's text content. Sorted so order doesn't matter.
fn project_for_snapshot(cont: &Value) -> Value {
    let mut tool_call_names: Vec<String> = Vec::new();
    let mut tool_result_texts: Vec<String> = Vec::new();

    fn walk(v: &Value, tool_call_names: &mut Vec<String>, tool_result_texts: &mut Vec<String>) {
        match v {
            Value::Object(obj) => {
                if let Some(name) = obj
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                {
                    tool_call_names.push(name.to_string());
                }
                if obj.get("role").and_then(|r| r.as_str()) == Some("tool") {
                    if let Some(content) = obj.get("content") {
                        if let Some(s) = content.as_str() {
                            tool_result_texts.push(s.to_string());
                        } else if let Some(arr) = content.as_array() {
                            for part in arr {
                                if let Some(s) = part.get("text").and_then(|t| t.as_str()) {
                                    tool_result_texts.push(s.to_string());
                                }
                            }
                        }
                    }
                }
                for (_, child) in obj {
                    walk(child, tool_call_names, tool_result_texts);
                }
            }
            Value::Array(arr) => {
                for child in arr {
                    walk(child, tool_call_names, tool_result_texts);
                }
            }
            _ => {}
        }
    }

    walk(cont, &mut tool_call_names, &mut tool_result_texts);
    tool_call_names.sort();
    tool_result_texts.sort();

    json!({
        "tool_calls": tool_call_names,
        "tool_results": tool_result_texts,
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_mcp_dispatch_round_trip() {
    // Skip-gate: no test API → nothing to talk to.
    if cli_test_util::test_api_address().is_none() {
        eprintln!("skipping plugin_mcp_dispatch_round_trip: OBJECTIVEAI_TEST_PORT not set");
        return;
    }

    // Build CLI + cli-stream + the plugin fixture once.
    let _ = cli_test_util::cli_binary();
    let _ = test_mcp_plugin_binary();

    let base = temp_base();
    let pid_file = base.join("plugin-pid");

    // Drop guard registered BEFORE we spawn anything that could
    // start the plugin. A mid-test panic still kills the plugin.
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
        base: base.clone(),
    };

    stage_plugin(&base);

    // Inline mock agent. ONLY `plugins[].mcp_servers` populated —
    // `tools`, `objectiveai`, and the plugin's own `executable`
    // flag are all left at the no-op default. This drives the CLI's
    // McpConfig header to:
    //   names = []
    //   objectiveai_builtins = false
    //   mcp_servers = [("test-mcp-plugin", "demo")]
    // which means `needs_primary = false` and ONLY the plugin
    // upstream gets dialed during `initialize`.
    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "client_objectiveai_mcp": {
            "plugins": [{
                "owner": "testorg",
                "name": "test-mcp-plugin",
                "version": "1.0.0",
                "executable": false,
                "mcp_servers": ["demo"]
            }]
        }
    });
    let agent = AgentSpec::Resolved(
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(agent_json)
            .expect("inline plugin-mcp agent must deserialize"),
    );

    // Executor pinned to this test's scratch base dir, with the
    // plugin's PID-file env var attached so the cli → cli-stream →
    // plugin chain inherits it.
    let executor = cli_test_util::executor_with_base_dir(&base)
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    // Spawn the agent. The plugin binary inherits OAI_TEST_MCP_PID_FILE
    // through the CLI subprocess → cli-stream → plugin chain.
    let spawn_request = SpawnRequest {
        prompt: RequestPrompt::Simple("use a tool".to_string()),
        agent,
        seed: Some(1),
        dangerous_advanced: None,
        jq: None,
    };
    let items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(&executor, spawn_request).await;
    let spawn_id = items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) => {
                if chunk.agent_instance_hierarchy.is_empty() {
                    None
                } else {
                    Some(chunk.agent_instance_hierarchy.clone())
                }
            }
            SpawnResponseItem::Id(_) => None,
        })
        .expect("agents spawn must emit a Chunk with agent_instance_hierarchy");

    wait_for_completion(&base, &spawn_id).await;

    let response_cont_path = base
        .join("logs/agents/completions/response/continuation")
        .join(format!("{spawn_id}.json"));
    let raw = std::fs::read_to_string(&response_cont_path).expect("read response continuation");
    let cont: Value = serde_json::from_str(&raw).expect("parse response continuation");

    let projection = project_for_snapshot(&cont);
    insta::assert_json_snapshot!("plugin_mcp_dispatch_round_trip", projection);

    // _guard drops here → kills plugin process → wipes scratch dir.
    drop(_guard);
}
