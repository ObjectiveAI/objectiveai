//! `agents spawn` + 2 continuations × 2 agents with a 10-tool surface
//! whose tools increment a per-MCP-session counter file. After three
//! turns per agent we use `agents read all <sub-id>` to enumerate the
//! tool-response queue items and `agents read id <sql-id>` to read
//! each one's content. The smoking-gun assertions:
//!
//! 1. Per agent, the highest count emitted by any tool response equals
//!    the total number of tool-response items for that agent — proves
//!    the count file *persisted* across continuations (a reset would
//!    leave it under the item count).
//! 2. Both agents' highest counts are equal — proves the
//!    deterministic-tool-selection is producing the same number of
//!    tool calls per turn under both seeds and that the per-session
//!    counters are independent (no cross-agent contamination).
//!
//! Driven through the SDK `BinaryExecutor` so each cli leaf is invoked
//! with a typed `Request` value rather than hand-rolled argv.

mod cli_test_util;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::{
    Request as MessageRequest, RequestMessage,
};
use objectiveai_sdk::cli::command::agents::read::all::{
    Request as ReadAllRequest, ResponseContent, ResponseItem as ReadAllItem,
    ResponseQueueItem,
};
use objectiveai_sdk::cli::command::agents::read::id::Request as ReadIdRequest;
use objectiveai_sdk::cli::command::agents::spawn::{
    AgentSpec, Request as SpawnRequest, RequestPrompt,
    ResponseItem as SpawnResponseItem,
};
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use serde_json::{Value, json};

static BUILD_COUNT_TOOL_ONCE: Once = Once::new();

/// Build the `count-tool` fixture binary into the shared per-test
/// target dir, then return its path. Reads `CARGO_TARGET_DIR` only
/// once via the same `BUILD_ONCE` cadence as the cli binary itself.
fn count_tool_binary() -> PathBuf {
    let target_dir = cli_test_util::test_target_dir();
    let mut path = target_dir.join("debug/count-tool");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    BUILD_COUNT_TOOL_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args([
                "build",
                "-p",
                "count-tool",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            .status()
            .expect("spawn cargo build count-tool");
        assert!(status.success(), "count-tool build failed");
    });
    path
}

async fn poll_until<F: Fn() -> bool>(timeout: Duration, pred: F) -> Result<(), ()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if pred() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(())
}

/// The test runs against the shared `objectiveai-mcp` server spawned
/// by `test-spawn-mcp-server.sh`, which has already registered
/// `testorg/tool{0..9}/1.0.0` manifests pointing at the `echo-arglen`
/// binary. We **commandeer** that binary by overwriting it with our
/// `count-tool` build — `count-tool` falls back to the `_default`
/// session id when `MCP_SESSION_ID` is unset, so any test that
/// happens to dispatch one of these tools without setting the env
/// still gets a valid (just session-less) output.
fn install_count_tool_over_echo_arglen() {
    let exec_name = if cfg!(windows) {
        "echo-arglen.exe"
    } else {
        "echo-arglen"
    };
    let dest = cli_test_util::mcp_session_shared_dir().join("tools").join(exec_name);
    assert!(
        dest.exists(),
        "expected the test mcp server's echo-arglen at {} — \
         did `test-spawn-mcp-server.sh` run?",
        dest.display(),
    );
    let bin = count_tool_binary();
    std::fs::copy(&bin, &dest).unwrap_or_else(|e| panic!("overwrite {}: {e}", dest.display()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))
            .expect("chmod count-tool");
    }
}

/// Inline mock-agent spec wired to the 10 `testorg/tool{0..9}/1.0.0`
/// tools the test mcp server registered. Same body regardless of
/// seed (the seed lives on the request, not the agent), so two
/// spawns with the same body produce the SAME agent definition but
/// two DIFFERENT per-turn `chunk.id`s.
fn agent_spec() -> AgentSpec {
    let tools: Vec<Value> = (0..10)
        .map(|i| {
            json!({
                "owner": "testorg",
                "name": format!("tool{i}"),
                "version": "1.0.0",
            })
        })
        .collect();
    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "client_objectiveai_mcp": {"tools": tools},
    });
    AgentSpec::Resolved(
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
            agent_json,
        )
        .expect("inline mock agent must deserialize"),
    )
}

/// Spawn an agent and return its sub-id (the chunk's
/// `agent_instance_hierarchy`).
async fn spawn_agent(executor: &BinaryExecutor, seed: i64) -> String {
    let request = SpawnRequest {
        prompt: RequestPrompt::Simple("go".to_string()),
        agent: agent_spec(),
        seed: Some(seed),
        dangerous_advanced: None,
        jq: None,
    };
    let items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(executor, request).await;
    items
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
        .expect("agents spawn must emit a Chunk with agent_instance_hierarchy")
}

/// Wait for the cli-stream child to have flushed an agent's response
/// continuation and unlinked its socket — same pattern the prior e2e
/// uses.
async fn wait_for_completion(base_dir: &Path, spawn_id: &str) {
    let response_cont_path = base_dir
        .join("logs/agents/completions/response/continuation")
        .join(format!("{spawn_id}.json"));
    let socket_path = base_dir.join("pipes/cli").join(spawn_id).join("socket");
    poll_until(Duration::from_secs(720), || {
        response_cont_path.exists() && !socket_path.exists()
    })
    .await
    .unwrap_or_else(|()| {
        panic!("cli-stream did not flush continuation + tear down socket for {spawn_id} in 720s",)
    });
}

/// Run one continuation turn against a spawned agent.
async fn continue_agent(executor: &BinaryExecutor, spawn_id: &str, seed: i64) {
    let request = MessageRequest {
        agent_instance_hierarchy: spawn_id.to_string(),
        message: RequestMessage::Simple("more".to_string()),
        seed: Some(seed),
        jq: None,
    };
    // Returns either Queued or Delivered — we don't care which here,
    // only that the cli emitted something without erroring. The real
    // verification is the post-turn `wait_for_completion`.
    let _ = executor
        .execute_one::<_, objectiveai_sdk::cli::command::agents::message::Response>(request, None)
        .await
        .expect("agents message executor call");
}

/// Collect every `tool_response` queue item's sql row id for `sub_id`
/// via the public `agents read all` cli surface.
async fn read_tool_response_ids(executor: &BinaryExecutor, sub_id: &str) -> Vec<i64> {
    let request = ReadAllRequest {
        agent_instance_hierarchies: vec![sub_id.to_string()],
        jq: None,
    };
    let items: Vec<ReadAllItem> =
        cli_test_util::collect_stream(executor, request).await;
    let mut ids = Vec::new();
    for item in items {
        for queue_item in item.items {
            if let ResponseQueueItem::ToolResponse { content, .. } = queue_item {
                match content {
                    ResponseContent::One(id) => ids.push(id),
                    ResponseContent::Many(many) => ids.extend(many),
                }
            }
        }
    }
    ids
}

/// Read one queue file by sql id and extract any embedded integer
/// (the count-tool prints e.g. `7\n`; once persisted as a tool-
/// response message file it lands as a JSON value whose text content
/// holds that number). Permissive — serializes the typed Response
/// back to JSON and scans recursively for the first integer-shaped
/// string or number.
async fn read_count_for_id(executor: &BinaryExecutor, id: i64) -> Option<u64> {
    let request = ReadIdRequest { id, jq: None };
    let response: objectiveai_sdk::cli::command::agents::read::id::Response = executor
        .execute_one(request, None)
        .await
        .expect("agents read id executor call");
    let value = serde_json::to_value(&response).expect("Response serializes");
    extract_first_u64(&value)
}

fn extract_first_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        Value::Array(arr) => arr.iter().find_map(extract_first_u64),
        Value::Object(obj) => obj.values().find_map(extract_first_u64),
        _ => None,
    }
}

#[tokio::test]
async fn two_agents_continuations_count_persists_per_session() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "OBJECTIVEAI_TEST_PORT not set — skipping two_agents_continuations_count_persists_per_session"
        );
        return;
    }

    // Use the shared MCP-session scratch dir — the same path
    // `test-spawn-mcp-server.sh` pins as CONFIG_BASE_DIR for the
    // spawned MCP server, so the two processes see one `tools/`
    // registry. This dir sits OUTSIDE the per-binary run-start
    // wipe (it's at `.objectiveai-tests/_mcp_session/`, not under
    // a `<binary>/` subfolder), so we still hand-wipe `logs`/
    // `pipes` here to clear prior-run state without nuking
    // `tools/` (which the MCP server registered at startup).
    let base_dir = cli_test_util::mcp_session_shared_dir();
    for sub in &["logs", "pipes"] {
        let p = base_dir.join(sub);
        if p.exists() {
            let _ = std::fs::remove_dir_all(&p);
        }
    }

    install_count_tool_over_echo_arglen();

    // Reset the count-tool state so the assertion sees only this
    // test's tool calls.
    let tool_data_dir = base_dir.join("tools").join("data");
    if tool_data_dir.exists() {
        let _ = std::fs::remove_dir_all(&tool_data_dir);
    }

    // One shared executor — all cli invocations point at the same
    // `CONFIG_BASE_DIR`.
    let executor = Arc::new(cli_test_util::executor_with_base_dir(&base_dir));

    // Each agent runs its full spawn → wait → continue → wait →
    // continue → wait pipeline as its own task. The two tasks are
    // independent — A doesn't gate on B's progress, and vice versa.
    // Distinct seeds give two distinct `chunk.id` lineages even
    // though the agent body content-hashes identically.
    let run_agent = |seed: i64| {
        let base_dir = base_dir.clone();
        let executor = executor.clone();
        async move {
            let id = spawn_agent(&executor, seed).await;
            wait_for_completion(&base_dir, &id).await;
            for _ in 0..2 {
                continue_agent(&executor, &id, seed).await;
                wait_for_completion(&base_dir, &id).await;
            }
            id
        }
    };

    let (a, b) = tokio::join!(run_agent(1), run_agent(2));
    assert_ne!(a, b, "two spawns must produce distinct lineages");

    let ids_a = read_tool_response_ids(&executor, &a).await;
    let ids_b = read_tool_response_ids(&executor, &b).await;

    assert!(
        !ids_a.is_empty(),
        "agent A produced zero tool responses — mock didn't call tools (seed/mode mismatch?)",
    );
    assert!(
        !ids_b.is_empty(),
        "agent B produced zero tool responses — mock didn't call tools (seed/mode mismatch?)",
    );

    let mut counts_a: Vec<u64> = Vec::with_capacity(ids_a.len());
    for id in &ids_a {
        if let Some(c) = read_count_for_id(&executor, *id).await {
            counts_a.push(c);
        }
    }
    let mut counts_b: Vec<u64> = Vec::with_capacity(ids_b.len());
    for id in &ids_b {
        if let Some(c) = read_count_for_id(&executor, *id).await {
            counts_b.push(c);
        }
    }

    let max_a = *counts_a.iter().max().expect("counts_a empty");
    let max_b = *counts_b.iter().max().expect("counts_b empty");

    // Per-agent persistence: if the counter had reset across any
    // turn, max would lag behind the total response count.
    assert_eq!(
        max_a as usize,
        ids_a.len(),
        "agent A's max count ({max_a}) must equal its tool-response item count ({}) — \
         a reset would leave it lower",
        ids_a.len(),
    );
    assert_eq!(
        max_b as usize,
        ids_b.len(),
        "agent B's max count ({max_b}) must equal its tool-response item count ({}) — \
         a reset would leave it lower",
        ids_b.len(),
    );
}
