//! End-to-end snapshot: `agents instances spawn` against an inline mock agent
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
use std::time::{Duration, Instant};

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::instances::spawn::{
    AgentSpec, Request as SpawnRequest, RequestDangerousAdvanced, ResponseItem as SpawnResponseItem,
};
use objectiveai_sdk::cli::command::agents::instances::message::RequestMessage;
use serde_json::{Value, json};

/// RAII kill of the plugin process (PID read from
/// `OAI_TEST_MCP_PID_FILE`) on test drop — success AND panic — so
/// the plugin RMCP server never leaks past the test boundary.
struct PluginGuard {
    pid_file: PathBuf,
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
    }
}

/// Wait for the CLI-stream child to have flushed the agent's
/// response continuation and torn down its socket.
///
/// On-disk conventions (see
/// `objectiveai-cli/src/filesystem/logs/log_file_kind.rs`):
///   continuation token (raw text, `.txt`):
///     `logs/agents/completions/response/continuation/<leaf>.txt`
///   per-agent socket:
///     `pipes/<full-lineage>/socket`
/// The continuation stems on the LEAF response id; the socket stems
/// on the FULL `agent_instance_hierarchy` (`cli/<leaf>` for a
/// caller-less cli invocation).
async fn wait_for_completion(base_dir: &Path, leaf: &str, full_lineage: &str) {
    let response_cont_path = base_dir
        .join("logs/agents/completions/response/continuation")
        .join(format!("{leaf}.txt"));
    let socket_path = base_dir.join("pipes").join(full_lineage).join("socket");
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if response_cont_path.exists() && !socket_path.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!(
        "cli-stream did not flush continuation + tear down socket for {leaf} (lineage {full_lineage}) in 180s",
    );
}

/// Extract a deterministic snapshot projection from the cli's
/// production log writer output for this completion. The top-level
/// envelope at `response/<leaf>.json` is just a list of
/// `{type: "reference", path: ...}` entries pointing at per-message
/// files in `response/messages/`; the snapshot-relevant content
/// (assistant tool call names, tool result text bodies) lives in
/// those per-message files, not in the envelope. So instead of
/// walking the envelope, glob the per-message log directories
/// directly:
///
/// - `response/messages/assistant/tool_calls/<leaf>_<msg>_<tc>.json`
///   carries `function.name` (one file per tool call within an
///   assistant message).
/// - `response/messages/tool/text/<leaf>_<msg>[_<part>].txt` carries
///   the raw text body of a `role: "tool"` response message (one
///   file per text part; single-part messages omit the `_<part>`
///   segment — see `LogFileKind::peel_text_stem`).
///
/// Both lists are sorted so message ordering / tool-call ordering
/// doesn't affect the snapshot — only the multiset of (name, text)
/// matters.
fn project_for_snapshot(base_dir: &Path, leaf: &str) -> Value {
    let leaf_prefix = format!("{leaf}_");

    let tool_calls_dir = base_dir
        .join("logs/agents/completions/response/messages/assistant/tool_calls");
    let mut tool_call_names: Vec<String> = std::fs::read_dir(&tool_calls_dir)
        .unwrap_or_else(|e| panic!(
            "read tool_calls dir {}: {e}",
            tool_calls_dir.display()
        ))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            // Each file is `<leaf>_<msg>_<tc>.json`. Only count files
            // whose stem starts with our leaf — the directory may
            // contain entries from other agents on a shared base dir.
            if !name.starts_with(&leaf_prefix) || !name.ends_with(".json") {
                return None;
            }
            let raw = std::fs::read_to_string(entry.path()).ok()?;
            let value: Value = serde_json::from_str(&raw).ok()?;
            value
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(str::to_string)
        })
        .collect();

    let tool_text_dir =
        base_dir.join("logs/agents/completions/response/messages/tool/text");
    let mut tool_result_texts: Vec<String> = std::fs::read_dir(&tool_text_dir)
        .unwrap_or_else(|e| panic!(
            "read tool/text dir {}: {e}",
            tool_text_dir.display()
        ))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(&leaf_prefix) || !name.ends_with(".txt") {
                return None;
            }
            std::fs::read_to_string(entry.path()).ok()
        })
        .collect();

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

    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");

    // Drop guard registered BEFORE we spawn anything that could
    // start the plugin. A mid-test panic still kills the plugin.
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };

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
                "mcp_servers": [{ "name": "demo" }]
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
    //
    // `dangerous_advanced.stream = true` keeps the parent cli attached
    // to its instance subprocess and forwards every
    // `AgentCompletionChunk` as `SpawnResponseItem::Chunk(_)` — we
    // need at least one Chunk to read `chunk.id` (the leaf response
    // id, which is what every on-disk log stem keys on). Without
    // streaming the parent cli detaches on `LogStreamReady` and emits
    // only a bare `Id(leaf)`.
    let spawn_request = SpawnRequest { path_type: objectiveai_sdk::cli::command::agents::instances::spawn::Path::AgentsInstancesSpawn,
        agent_tag: None,
        message: RequestMessage::Simple("use a tool".to_string()),
        agent,
        seed: Some(1),
        dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
        jq: None,
    };
    let items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(&executor, spawn_request).await;
    // Pull the leaf response id off the first non-empty Chunk.
    //
    // We deliberately ignore `chunk.agent_instance_hierarchy`: the API
    // emits it as `{caller}/{agent_full_id}-{response_id}` (the
    // api-side slot identifier), but the cli's on-disk filesystem
    // stores rows under `{caller}/{response_id_leaf}` (the cli-side
    // lineage). The cli-side full lineage is `cli/<leaf>` for a
    // caller-less invocation.
    let leaf = items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) => {
                if chunk.id.is_empty() {
                    None
                } else {
                    Some(chunk.id.clone())
                }
            }
            SpawnResponseItem::Id(_) => None,
        })
        .expect("agents instances spawn must emit a Chunk with non-empty id");
    let full_lineage = format!("cli/{leaf}");

    wait_for_completion(&base, &leaf, &full_lineage).await;

    // `project_for_snapshot` reads from the cli's production log
    // writer output for THIS completion: tool-call names from
    // `response/messages/assistant/tool_calls/<leaf>_*_*.json` and
    // tool-result text from
    // `response/messages/tool/text/<leaf>_*.txt`. See its doc for
    // why we don't walk the top-level envelope (it only carries
    // `{type: "reference", path: ...}` entries).
    let projection = project_for_snapshot(&base, &leaf);
    insta::assert_json_snapshot!("plugin_mcp_dispatch_round_trip", projection);

    // _guard drops here → kills plugin process → wipes scratch dir.
    drop(_guard);
}
