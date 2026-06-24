//! End-to-end: a plugin MCP tool re-enters the per-`response_id`
//! socket mechanic. An inline mock agent calls a tool on the
//! `test-mcp-plugin-self-call` plugin; that tool reads its
//! `X-OBJECTIVEAI-RESPONSE-ID` header and, via the objectiveai
//! `PluginExecutor`, re-invokes `agents tools|resources …` for that
//! response id — which routes back through the host's listener socket
//! into the same agent's MCP aggregation. The tool returns the MCP
//! result, which we assert from `objectiveai.tool_response_content_text`.
//!
//! Four surfaces (selected by mcp-server name), one per test:
//! - `call-other`     — `call_hello` calls the sibling `hello` tool
//!   through the system; result contains "hello world".
//! - `list-tools`     — `do_list_tools` returns `agents tools list`.
//! - `list-resources` — `do_list_resources` returns `agents resources list`.
//! - `read-resource`  — `do_read_resource` returns `agents resources read`.
//!
//! The plugin RMCP server is killed before each test returns via a
//! `Drop` guard reading the PID from `OAI_TEST_MCP_PID_FILE`.

mod cli_test_util;

use std::path::PathBuf;
use std::process::Command;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::RequestMessage;
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn::{
    Request as SpawnRequest, RequestDangerousAdvanced, ResponseItem as SpawnResponseItem,
};
use serde_json::json;

/// RAII kill of the plugin process (PID read from
/// `OAI_TEST_MCP_PID_FILE`) on test drop — success AND panic.
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

/// Pull every `tool_response_content_text.text` row for `response_id`.
async fn tool_result_texts<E>(executor: &E, response_id: &str) -> Vec<String>
where
    E: objectiveai_sdk::cli::command::CommandExecutor,
    E::Error: std::fmt::Debug,
{
    let sql = format!(
        "SELECT text FROM objectiveai.tool_response_content_text \
         WHERE response_id = '{}' ORDER BY \"index\", part_index",
        response_id.replace('\'', "''"),
    );
    let rows = cli_test_util::db_query(executor, &sql).await;
    rows.into_iter()
        .filter_map(|mut row| row.pop())
        .filter_map(|v| match v {
            serde_json::Value::String(s) => Some(s),
            _ => None,
        })
        .collect()
}

/// Drive one surface end to end and return the joined tool-result text.
/// `mcp_server` selects the plugin surface; `entry_tool` is the
/// aggregated tool name the mock is scripted to call.
async fn run_surface(mcp_server: &str, entry_tool: &str) -> String {
    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };

    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "client_objectiveai_mcp": {
            "plugins": [{
                "owner": "testorg",
                "name": "test-mcp-plugin-self-call",
                "version": "1.0.0",
                "executable": false,
                "mcp_servers": [{ "name": mcp_server }]
            }]
        },
        // Deterministic script: one assistant turn that calls the entry
        // tool with empty args, then stops.
        "calls": [{
            "tool_calls": [{ "name": entry_tool, "arguments": "{}" }],
            "content": ""
        }]
    });
    let agent = AgentSelector::Ref {
        agent: AgentRef::Resolved(
            serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                agent_json,
            )
            .expect("inline self-call agent must deserialize"),
        ),
    };

    let executor = cli_test_util::executor().await.env(
        "OAI_TEST_MCP_PID_FILE",
        pid_file.to_string_lossy().into_owned(),
    );

    let spawn_request = SpawnRequest {
        path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
        message: RequestMessage::Simple("use a tool".to_string()),
        agent,
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(1),
        }),
        base: Default::default(),
    };
    let items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(&executor, spawn_request).await;
    let full_aih = items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk)
                if !chunk.agent_instance_hierarchy.is_empty() =>
            {
                Some(chunk.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("agents spawn must emit a Chunk with a non-empty agent_instance_hierarchy");
    let response_id = items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) if !chunk.id.is_empty() => Some(chunk.id.clone()),
            _ => None,
        })
        .expect("agents spawn must emit a Chunk with a non-empty id");
    cli_test_util::wait_for_agent(&executor, &full_aih).await;

    tool_result_texts(&executor, &response_id).await.join("\n")
}

#[tokio::test(flavor = "multi_thread")]
async fn self_call_tools_call_round_trip() {
    let results = run_surface(
        "call-other",
        "test-mcp-plugin-self-call_call_hello",
    )
    .await;
    assert!(
        results.contains("hello world"),
        "call_hello should return the sibling hello tool's output; got: {results}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn self_call_tools_list_round_trip() {
    let results = run_surface(
        "list-tools",
        "test-mcp-plugin-self-call_do_list_tools",
    )
    .await;
    assert!(
        results.contains("do_list_tools"),
        "tools list should include the plugin's own tool; got: {results}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn self_call_resources_list_round_trip() {
    let results = run_surface(
        "list-resources",
        "test-mcp-plugin-self-call_do_list_resources",
    )
    .await;
    assert!(
        results.contains("hello-resource"),
        "resources list should include the declared resource; got: {results}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn self_call_resources_read_round_trip() {
    let results = run_surface(
        "read-resource",
        "test-mcp-plugin-self-call_do_read_resource",
    )
    .await;
    assert!(
        results.contains("resource hello world"),
        "resources read should return the declared resource content; got: {results}"
    );
}
