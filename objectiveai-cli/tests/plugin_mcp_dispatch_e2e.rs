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
//! Skip-gate: requires `OBJECTIVEAI_TEST_PORT` to point at a
//! running test API (mirrors the existing snapshot tests).

mod cli_test_util;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

static BUILD_CLI_STREAM_ONCE: Once = Once::new();
static BUILD_TEST_MCP_PLUGIN_ONCE: Once = Once::new();

fn ensure_cli_stream_built() {
    BUILD_CLI_STREAM_ONCE.call_once(|| {
        let target_dir = cli_test_util::test_target_dir();
        let status = Command::new("cargo")
            .args([
                "build",
                "-p",
                "objectiveai-cli-stream",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            .status()
            .expect("spawn cargo build cli-stream");
        assert!(status.success(), "cli-stream build failed");
    });
}

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
    let d = std::env::temp_dir()
        .join(format!("oai-mcp-plugin-{}", uuid::Uuid::new_v4()));
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
        std::fs::set_permissions(
            &installed,
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
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
                    let _ = Command::new("kill")
                        .args(["-9", &pid.to_string()])
                        .status();
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
    let socket_path = base_dir
        .join("pipes/cli")
        .join(spawn_id)
        .join("socket");
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if response_cont_path.exists() && !socket_path.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!(
        "cli-stream did not flush continuation + tear down socket for {spawn_id} in 180s",
    );
}

fn cli_command_with_base_dir(base_dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(cli_test_util::cli_binary());
    cmd.env("CONFIG_BASE_DIR", base_dir);
    if let Some(addr) = cli_test_util::test_api_address() {
        cmd.env("OBJECTIVEAI_ADDRESS", addr);
    }
    cmd.args(args);
    cmd
}

/// Extract a deterministic snapshot projection from the response
/// continuation JSON: every assistant tool call's `name`, and every
/// tool-result message's text content. Sorted so order doesn't matter.
fn project_for_snapshot(cont: &Value) -> Value {
    let mut tool_call_names: Vec<String> = Vec::new();
    let mut tool_result_texts: Vec<String> = Vec::new();

    fn walk(
        v: &Value,
        tool_call_names: &mut Vec<String>,
        tool_result_texts: &mut Vec<String>,
    ) {
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
                                if let Some(s) =
                                    part.get("text").and_then(|t| t.as_str())
                                {
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
    ensure_cli_stream_built();
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
    let agent = json!({
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
    })
    .to_string();

    // Spawn the agent. The plugin binary inherits OAI_TEST_MCP_PID_FILE
    // through the CLI subprocess → cli-stream → plugin chain.
    let mut spawn_cmd = cli_command_with_base_dir(
        &base,
        &[
            "agents", "spawn",
            "--agent-inline", &agent,
            "--simple", "use a tool",
            "--seed", "1",
        ],
    );
    spawn_cmd.env("OAI_TEST_MCP_PID_FILE", &pid_file);
    let output = spawn_cmd.output().expect("execute cli");
    if !output.status.success() {
        panic!(
            "cli agents spawn exited with {}\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    let lines: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .collect();
    let spawned = lines
        .iter()
        .find(|l| l.pointer("/value/kind") == Some(&json!("spawned")))
        .expect("agents spawn must emit Spawned notification");
    let spawn_id = spawned
        .pointer("/value/agent_id")
        .and_then(|v| v.as_str())
        .expect("Spawned.agent_id")
        .to_string();

    wait_for_completion(&base, &spawn_id).await;

    let response_cont_path = base
        .join("logs/agents/completions/response/continuation")
        .join(format!("{spawn_id}.json"));
    let raw = std::fs::read_to_string(&response_cont_path)
        .expect("read response continuation");
    let cont: Value = serde_json::from_str(&raw).expect("parse response continuation");

    let projection = project_for_snapshot(&cont);
    insta::assert_json_snapshot!("plugin_mcp_dispatch_round_trip", projection);

    // _guard drops here → kills plugin process → wipes scratch dir.
    drop(_guard);
}
