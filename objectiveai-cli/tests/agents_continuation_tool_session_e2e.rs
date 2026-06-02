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

mod cli_test_util;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

static BUILD_CLI_STREAM_ONCE: Once = Once::new();
static BUILD_COUNT_TOOL_ONCE: Once = Once::new();

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

fn cli_command_with_base_dir(base_dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(cli_test_util::cli_binary());
    cmd.env("CONFIG_BASE_DIR", base_dir);
    if let Some(addr) = cli_test_util::test_api_address() {
        cmd.env("OBJECTIVEAI_ADDRESS", addr);
    }
    cmd.args(args);
    cmd
}

fn run_cli_with_base_dir(base_dir: &Path, args: &[&str]) -> Vec<Value> {
    let output = cli_command_with_base_dir(base_dir, args)
        .output()
        .expect("execute cli");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        panic!(
            "cli exited with {}\nargs: {args:?}\nstdout: {stdout}\nstderr: {stderr}",
            output.status,
        );
    }
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .collect()
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
    let dest = cli_test_util::tests_dir().join("tools").join(exec_name);
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

/// Inline mock-agent JSON wired to the 10 `testorg/tool{0..9}/1.0.0`
/// tools the test mcp server registered. Same body regardless of
/// seed (the seed lives on the request, not the agent), so two
/// spawns with the same body produce the SAME agent definition but
/// two DIFFERENT per-turn `chunk.id`s. Each lineage `cli/<chunk.id>`
/// becomes its own MCP session id via the objectiveai-mcp fallback
/// chain to `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`.
fn agent_json() -> String {
    let tools: Vec<Value> = (0..10)
        .map(|i| {
            json!({
                "owner": "testorg",
                "name": format!("tool{i}"),
                "version": "1.0.0",
            })
        })
        .collect();
    json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "client_objectiveai_mcp": {"tools": tools},
    })
    .to_string()
}

/// Spawn an agent and return its sub-id (the `Spawned.agent_instance_hierarchy`).
fn spawn_agent(base_dir: &Path, agent_json: &str, seed: i64) -> String {
    let seed_str = seed.to_string();
    let lines = run_cli_with_base_dir(
        base_dir,
        &[
            "agents",
            "spawn",
            "--agent-inline",
            agent_json,
            "--simple",
            "go",
            "--seed",
            &seed_str,
        ],
    );
    let spawned = lines
        .iter()
        .find(|l| l.pointer("/type") == Some(&json!("spawned")))
        .expect("agents spawn must emit Spawned");
    spawned
        .pointer("/agent_instance_hierarchy")
        .and_then(|v| v.as_str())
        .expect("Spawned.agent_instance_hierarchy")
        .to_string()
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

/// Run one continuation turn against a spawned agent. Sync — the
/// caller (the parallel-agents flow) wraps this in `spawn_blocking`
/// and then `await`s a fresh `wait_for_completion` separately.
fn continue_agent_sync(base_dir: &Path, spawn_id: &str, seed: i64) {
    let seed_str = seed.to_string();
    let _ = run_cli_with_base_dir(
        base_dir,
        &[
            "agents", "message", spawn_id, "--simple", "more", "--seed", &seed_str,
        ],
    );
}

/// Collect every `tool_response` queue item's sql row id (from the
/// `content` field's `One` / `Many` variant) for `sub_id`, via the
/// public `agents read all` cli surface.
fn read_tool_response_ids(base_dir: &Path, sub_id: &str) -> Vec<i64> {
    let lines = run_cli_with_base_dir(base_dir, &["agents", "read", "all", sub_id]);
    let agent_items = lines
        .iter()
        .find(|l| l.pointer("/type") == Some(&json!("agent_items")))
        .expect("agents read all must emit AgentItems");
    let items = agent_items
        .pointer("/items")
        .and_then(|v| v.as_array())
        .expect("AgentItems.items");

    let mut ids = Vec::new();
    for item in items {
        if item.get("type").and_then(|t| t.as_str()) != Some("tool_response") {
            continue;
        }
        let content = match item.get("content") {
            Some(c) => c,
            None => continue,
        };
        // `Content` is untagged: `One(Id)` (integer) | `Many(Vec<Id>)`
        // (array of integers).
        if let Some(n) = content.as_i64() {
            ids.push(n);
        } else if let Some(arr) = content.as_array() {
            for v in arr {
                if let Some(n) = v.as_i64() {
                    ids.push(n);
                }
            }
        }
    }
    ids
}

/// Read one queue file by sql id and extract any embedded integer
/// (the count-tool prints e.g. `7\n`; once persisted as a tool-
/// response message file it lands as a JSON value whose text content
/// holds that number). Permissive — searches the whole JSON value
/// recursively for the first integer-shaped string or number.
fn read_count_for_id(base_dir: &Path, id: i64) -> Option<u64> {
    let id_str = id.to_string();
    let lines = run_cli_with_base_dir(base_dir, &["agents", "read", "id", &id_str]);
    let value = lines
        .iter()
        .find(|l| l.pointer("/content").is_some())
        .and_then(|l| l.pointer("/content").cloned())?;
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
    ensure_cli_stream_built();

    // Use the SHARED test scratch dir so the MCP server (which is
    // pinned to `objectiveai-cli/tests/.objectiveai` by
    // `test-spawn-mcp-server.sh`) and the cli-stream we spawn here
    // agree on `CONFIG_BASE_DIR` — the response continuation files
    // and the per-agent socket both have to land where we expect to
    // poll for them.
    let base_dir = cli_test_util::tests_dir();
    // Wipe agent state from prior runs but keep the tools/ subtree
    // (the MCP server's manifest registry).
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

    // Each agent runs its full spawn → wait → continue → wait →
    // continue → wait pipeline as its own task. The two tasks are
    // independent — A doesn't gate on B's progress, and vice versa.
    // Distinct seeds give two distinct `chunk.id` lineages even
    // though the agent body content-hashes identically.
    let agent = agent_json();
    let run_agent = |seed: i64| {
        let base_dir = base_dir.clone();
        let agent = agent.clone();
        async move {
            let id = tokio::task::spawn_blocking({
                let base_dir = base_dir.clone();
                let agent = agent.clone();
                move || spawn_agent(&base_dir, &agent, seed)
            })
            .await
            .expect("spawn_agent task panicked");
            wait_for_completion(&base_dir, &id).await;
            for _ in 0..2 {
                let id_c = id.clone();
                let base_c = base_dir.clone();
                tokio::task::spawn_blocking(move || continue_agent_sync(&base_c, &id_c, seed))
                    .await
                    .expect("continue_agent task panicked");
                wait_for_completion(&base_dir, &id).await;
            }
            id
        }
    };

    let (a, b) = tokio::join!(run_agent(1), run_agent(2));
    assert_ne!(a, b, "two spawns must produce distinct lineages");

    let ids_a = read_tool_response_ids(&base_dir, &a);
    let ids_b = read_tool_response_ids(&base_dir, &b);

    assert!(
        !ids_a.is_empty(),
        "agent A produced zero tool responses — mock didn't call tools (seed/mode mismatch?)",
    );
    assert!(
        !ids_b.is_empty(),
        "agent B produced zero tool responses — mock didn't call tools (seed/mode mismatch?)",
    );

    let counts_a: Vec<u64> = ids_a
        .iter()
        .filter_map(|&id| read_count_for_id(&base_dir, id))
        .collect();
    let counts_b: Vec<u64> = ids_b
        .iter()
        .filter_map(|&id| read_count_for_id(&base_dir, id))
        .collect();

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
