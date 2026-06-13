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
//! its URL.
//!
//! Driven through the SDK `BinaryExecutor`. Snapshot inputs come
//! from postgres via `db query`:
//! - `tool_calls`  — `function.name` for every assistant tool-call
//!   (extracted from `objectiveai.agent_completion_responses.body`).
//! - `tool_results` — `text` rows from
//!   `objectiveai.tool_response_content_text` for this response_id.

mod cli_test_util;

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::logs::read::all::{
    AssistantResponsePartType, Request as ReadAllRequest, ResponseItem as ReadAllItem,
    Target as ReadAllTarget,
};
use objectiveai_sdk::cli::command::agents::message::RequestMessage;
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn::{
    Request as SpawnRequest, RequestDangerousAdvanced,
    ResponseItem as SpawnResponseItem,
};
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

/// Pull every `tool_response_content_text.text` row for the given
/// `response_id` and sort it. Order varies based on event scheduling
/// — the snapshot is multiset-based, not ordered.
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

#[tokio::test(flavor = "multi_thread")]
async fn plugin_mcp_dispatch_round_trip() {
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
                "name": "test-mcp-plugin",
                "version": "1.0.0",
                "executable": false,
                "mcp_servers": [{ "name": "demo" }]
            }]
        }
    });
    let agent = AgentSelector::Ref {
        agent: AgentRef::Resolved(
            serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                agent_json,
            )
            .expect("inline plugin-mcp agent must deserialize"),
        ),
    };

    let executor = cli_test_util::executor().await
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    let spawn_request = SpawnRequest {
        path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
        message: RequestMessage::Simple("use a tool".to_string()),
        agent,
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(1),
            skip_lock: None,
        }),
        base: Default::default(),
    };
    let items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(&executor, spawn_request).await;
    let full_aih = items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) if !chunk.agent_instance_hierarchy.is_empty() => {
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
    cli_test_util::wait_for_continuation(&executor, &full_aih, Duration::from_secs(180)).await;

    // `tool_calls`: every assistant `AssistantResponsePart` of
    // type `ToolCall` carries its `function_name` (sourced from
    // `objectiveai.assistant_response_tool_calls.function_name`, which
    // is per-row INSERT-then-UPDATE — accumulates across turns
    // and survives continuation, unlike the response blob).
    // `tool_results`: every `objectiveai.tool_response_content_text`
    // row for this `response_id` (append-only).
    let target_instance = full_aih
        .rsplit_once('/')
        .map(|(_, i)| i.to_string())
        .unwrap_or_else(|| full_aih.clone());
    let read_all = ReadAllRequest {
        path_type: objectiveai_sdk::cli::command::agents::logs::read::all::Path::AgentsLogsReadAll,
        targets: vec![ReadAllTarget::Direct {
            parent_agent_instance_hierarchy: None,
            agent_instance: target_instance,
        }],
        after_id: None,
        limit: None,
        base: Default::default(),
    };
    let blocks: Vec<ReadAllItem> = cli_test_util::collect_stream(&executor, read_all).await;
    let mut tool_call_names: Vec<String> = Vec::new();
    for block in &blocks {
        if let ReadAllItem::AssistantResponse { parts, .. } = block {
            for part in parts {
                if matches!(part.r#type, AssistantResponsePartType::ToolCall) {
                    tool_call_names.push(part.function_name.clone());
                }
            }
        }
    }
    tool_call_names.sort();
    let mut tool_result_texts = tool_result_texts(&executor, &response_id).await;
    tool_result_texts.sort();

    let projection: Value = json!({
        "tool_calls": tool_call_names,
        "tool_results": tool_result_texts,
    });
    insta::assert_json_snapshot!("plugin_mcp_dispatch_round_trip", projection);

    drop(_guard);
}
