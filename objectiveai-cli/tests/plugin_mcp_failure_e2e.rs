//! End-to-end: `agents spawn` against an inline mock agent whose only
//! MCP surface is the `test-mcp-plugin-failing` RMCP plugin, which
//! injects a failure at one chosen stage (`--fail-at
//! connect|list|call`, threaded through the agent's
//! `mcp_servers[].arguments`). Four tests, one per failure shape, each
//! in its own `OBJECTIVEAI_STATE` (= test-fn name):
//!
//!  1. `connect` — the plugin's `initialize` errors. The conduit dial
//!     fails; the single fallback-less agent has nowhere to go, so the
//!     spawn surfaces an error and produces no completion chunk.
//!  2. `list` — `initialize` succeeds but `list_tools` errors, caught
//!     by the proxy's connect-time health probe. Same surfacing as (1).
//!  3. `call` (tool exists) — the agent calls the real `failsvr_doit`;
//!     the upstream `call_tool` errors, the loop synthesizes an error
//!     tool response and keeps going. The spawn succeeds; the logs show
//!     exactly one tool call and one tool response.
//!  4. `call` (tool missing) — the agent calls `failsvr_nope`, a name
//!     the proxy never advertised. The api's `extract_callable_tool_calls`
//!     filters it before dispatch, so the loop breaks with no tool
//!     response. The logs show one tool call and zero tool responses.
//!
//! Fast-fail enabler: tests 1 & 2 hinge on a deterministic MCP failure
//! erroring *quickly* — the conduit's MCP client otherwise retries every
//! failure for its whole backoff window. Each test first writes a
//! very-low `api.mcp_backoff` into its own state config, which the
//! conduit reads (and, when this test happens to spawn the shared api,
//! is also projected onto the api's `MCP_BACKOFF_*` env). The shared
//! api/proxy side is independently kept fast by the test `.env`'s
//! `MCP_BACKOFF_*=0`.
//!
//! Plugin lifecycle: the plugin writes its PID to `OAI_TEST_MCP_PID_FILE`
//! before announcing its URL; a `Drop` guard force-kills it so its RMCP
//! server never leaks past the test boundary.

mod cli_test_util;

use std::path::PathBuf;
use std::process::Command;

use futures::StreamExt;
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::agents::instances::list::{
    Path as InstancesPath, Request as InstancesRequest, ResponseItem as InstancesItem,
};
use objectiveai_sdk::cli::command::agents::logs::read::all::{
    AssistantResponsePart, Path as ReadAllPath, Request as ReadAllRequest,
    ResponseItem as ReadAllItem, Target as ReadAllTarget,
};
use objectiveai_sdk::cli::command::agents::message::RequestMessage;
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn::{
    Path as SpawnPath, Request as SpawnRequest, RequestDangerousAdvanced,
    ResponseItem as SpawnResponseItem,
};
use serde_json::{Value, json};

/// RAII kill of the plugin process (PID read from
/// `OAI_TEST_MCP_PID_FILE`) on test drop — success AND panic — so the
/// plugin RMCP server never leaks past the test boundary.
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

/// Write a very-low `api.mcp_backoff` into this test's state config so a
/// deterministic MCP failure errors in ~1ms instead of retrying for the
/// default ~40s window — which would stall a fallback-less agent with no
/// output and trip the harness's hang watchdog. `max_elapsed_time_ms: 1`
/// means the conduit's MCP client gives up effectively after the first
/// attempt. Written via the filesystem `Client` directly (not the cli
/// command), mirroring `viewer_send_e2e`.
async fn set_fast_mcp_backoff() {
    let fs = objectiveai_cli::filesystem::Client::new(
        Some(cli_test_util::objectiveai_dir()),
        Some(cli_test_util::test_state_name()),
        None::<String>,
        None::<String>,
    );
    let mut config = fs.read_config().await.expect("read_config failed");
    config.api().set_mcp_backoff(objectiveai_sdk::mcp::Backoff {
        connect_timeout_ms: 5_000,
        current_interval_ms: 1,
        initial_interval_ms: 1,
        randomization_factor: 0.0,
        multiplier: 1.0,
        max_interval_ms: 1,
        max_elapsed_time_ms: 1,
        call_timeout_ms: 5_000,
    });
    fs.write_config(&config).await.expect("write_config failed");
}

/// Inline mock-agent JSON pointed at the failing plugin's `demo` MCP,
/// with `--fail-at <fail_at>` threaded through `mcp_servers[].arguments`
/// and the deterministic `calls` script supplied by the caller.
/// `executable: false` keeps it MCP-only.
fn failing_agent(fail_at: &str, calls: Value) -> Value {
    json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "client_objectiveai_mcp": {
            "plugins": [{
                "owner": "testorg",
                "name": "test-mcp-plugin-failing",
                "version": "1.0.0",
                "executable": false,
                "mcp_servers": [{
                    "name": "demo",
                    "arguments": { "fail-at": fail_at }
                }]
            }]
        },
        "calls": calls
    })
}

fn agent_selector(agent_json: Value) -> AgentSelector {
    AgentSelector::Ref {
        agent: AgentRef::Resolved(
            serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                agent_json,
            )
            .expect("inline failing-plugin agent must deserialize"),
        ),
    }
}

fn spawn_request(agent: AgentSelector) -> SpawnRequest {
    SpawnRequest {
        path_type: SpawnPath::AgentsSpawn,
        message: RequestMessage::Simple("use a tool".to_string()),
        agent,
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(1),
            skip_lock: None,
        }),
        base: Default::default(),
    }
}

/// Drive a streaming spawn that is expected to FAIL at MCP init
/// (connect or list). For a fallback-less agent, that surfaces as a
/// stream-level `Err` item (the api sends a `ResponseError` frame; the
/// cli re-emits it as an error line, which `BinaryExecutor` parses into
/// an `Err`) — NOT as a collectable chunk — so we drive the raw stream
/// rather than `collect_stream` (which panics on `Err`). Asserts an
/// error surfaced and that no completion chunk was produced (the agent
/// never ran).
async fn assert_spawn_errors_without_running(
    executor: &cli_test_util::HangPreventingBinaryCommandExecutor,
    agent_json: Value,
) {
    let request = spawn_request(agent_selector(agent_json));
    let stream = executor
        .execute::<SpawnRequest, SpawnResponseItem>(request, None)
        .await
        .expect("cli execute must start");
    let mut stream = std::pin::pin!(stream);
    let mut saw_err = false;
    let mut chunks = 0usize;
    while let Some(item) = stream.next().await {
        match item {
            Ok(SpawnResponseItem::Chunk(chunk)) => {
                chunks += 1;
                // An error carried on a chunk also counts as "errored".
                if chunk.error.is_some() {
                    saw_err = true;
                }
            }
            Ok(SpawnResponseItem::Id(_)) => {}
            Err(_) => saw_err = true,
        }
    }
    assert!(
        saw_err,
        "spawn must surface an error for a plugin that fails at MCP init",
    );
    assert_eq!(
        chunks, 0,
        "a fallback-less agent whose MCP init fails must not emit any completion chunk",
    );
}

/// Spawn a streaming agent that runs to completion, then read its logs
/// via the path the test spec dictates: `agents instances list --target
/// me` → take the first instance → `agents logs read all` on its AIH.
/// Returns `(tool_call_parts, tool_response_blocks)`.
async fn spawn_and_count_tool_io(
    executor: &cli_test_util::HangPreventingBinaryCommandExecutor,
    agent_json: Value,
) -> (usize, usize) {
    let items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(executor, spawn_request(agent_selector(agent_json))).await;
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
        .expect("spawn must emit a Chunk carrying a non-empty agent_instance_hierarchy");
    cli_test_util::wait_for_agent(executor, &full_aih).await;

    // `agents instances list --target me` lists the direct children of
    // the cli's own AIH (`cli`); the spawn created exactly one. Take the
    // first and read its logs — the path the spec calls for.
    let instances: Vec<InstancesItem> = cli_test_util::collect_stream(
        executor,
        InstancesRequest {
            path_type: InstancesPath::AgentsInstancesList,
            targets: vec![ReadAllTarget::Me],
            base: Default::default(),
        },
    )
    .await;
    let listed_aih = instances
        .first()
        .expect("instances list must return the spawned agent")
        .agent_instance_hierarchy
        .clone();

    let (parent, instance) = listed_aih
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or((None, listed_aih.clone()));
    let blocks: Vec<ReadAllItem> = cli_test_util::collect_stream(
        executor,
        ReadAllRequest {
            path_type: ReadAllPath::AgentsLogsReadAll,
            targets: vec![ReadAllTarget::Direct {
                parent_agent_instance_hierarchy: parent,
                agent_instance: instance,
            }],
            after_id: None,
            limit: None,
            base: Default::default(),
        },
    )
    .await;

    let mut tool_calls = 0usize;
    let mut tool_responses = 0usize;
    for block in &blocks {
        match block {
            ReadAllItem::AssistantResponse { parts, .. } => {
                for part in parts {
                    if matches!(part, AssistantResponsePart::ToolCall { .. }) {
                        tool_calls += 1;
                    }
                }
            }
            ReadAllItem::ToolResponse { .. } => tool_responses += 1,
            _ => {}
        }
    }
    (tool_calls, tool_responses)
}

/// Test 1: the plugin errors on `initialize`. The agent has no fallbacks,
/// so the spawn errors out and never runs.
#[tokio::test(flavor = "multi_thread")]
async fn plugin_mcp_connect_failure_errors_out() {
    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };
    set_fast_mcp_backoff().await;

    let executor = cli_test_util::executor()
        .await
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    let agent = failing_agent("connect", json!([{ "tool_calls": [], "content": "" }]));
    assert_spawn_errors_without_running(&executor, agent).await;

    drop(_guard);
}

/// Test 2: `initialize` succeeds, but `list_tools` errors (caught by the
/// proxy's connect-time health probe). Same outcome as test 1.
#[tokio::test(flavor = "multi_thread")]
async fn plugin_mcp_list_failure_errors_out() {
    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };
    set_fast_mcp_backoff().await;

    let executor = cli_test_util::executor()
        .await
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    let agent = failing_agent("list", json!([{ "tool_calls": [], "content": "" }]));
    assert_spawn_errors_without_running(&executor, agent).await;

    drop(_guard);
}

/// Test 3: connect + list succeed; the agent calls the real
/// `failsvr_doit`, whose `call_tool` errors. The loop synthesizes an
/// error tool response and continues to a final content turn. The spawn
/// succeeds; the logs carry exactly one tool call and one tool response
/// (content not checked).
#[tokio::test(flavor = "multi_thread")]
async fn plugin_mcp_call_failure_records_tool_response() {
    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };
    set_fast_mcp_backoff().await;

    let executor = cli_test_util::executor()
        .await
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    // Turn 1 calls the real (listed) tool; turn 2 closes out with content
    // so the loop breaks cleanly after the synthesized error response.
    let agent = failing_agent(
        "call",
        json!([
            {
                "tool_calls": [{ "name": "failsvr_doit", "arguments": "{}" }],
                "content": ""
            },
            { "tool_calls": [], "content": "done" }
        ]),
    );
    let (tool_calls, tool_responses) = spawn_and_count_tool_io(&executor, agent).await;
    assert_eq!(
        tool_calls, 1,
        "expected exactly one assistant tool call (failsvr_doit)",
    );
    assert_eq!(
        tool_responses, 1,
        "expected exactly one tool response (the synthesized call-tool error)",
    );

    drop(_guard);
}

/// Test 4: same call-failing plugin, but the agent calls `failsvr_nope`,
/// a name the proxy never advertised. `extract_callable_tool_calls`
/// filters it before dispatch, so no tool response is produced and the
/// loop breaks. The logs carry one tool call and zero tool responses.
#[tokio::test(flavor = "multi_thread")]
async fn plugin_mcp_unlisted_tool_call_has_no_response() {
    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };
    set_fast_mcp_backoff().await;

    let executor = cli_test_util::executor()
        .await
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    let agent = failing_agent(
        "call",
        json!([{
            "tool_calls": [{ "name": "failsvr_nope", "arguments": "{}" }],
            "content": ""
        }]),
    );
    let (tool_calls, tool_responses) = spawn_and_count_tool_io(&executor, agent).await;
    assert_eq!(
        tool_calls, 1,
        "expected exactly one assistant tool call (failsvr_nope)",
    );
    assert_eq!(
        tool_responses, 0,
        "an unlisted tool name is filtered before dispatch — no tool response",
    );

    drop(_guard);
}
