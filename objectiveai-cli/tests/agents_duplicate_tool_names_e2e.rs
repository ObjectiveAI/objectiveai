//! Five plugin installs, each pointing at the same `test-mcp-plugin-named`
//! fixture binary launched with a distinct `--name` argv. Each plugin's
//! upstream advertises a single tool called `invoke`; the proxy prefixes
//! each one with the upstream's `serverInfo.name` (which the fixture
//! echoes from `--name`), so the agent sees five distinct tool names
//! that all share an inner `invoke`.
//!
//! A bare-bones plain mock agent then runs three CLI turns against the
//! same `agent_instance_hierarchy`: one `agents spawn`, two `agents
//! message`. After every turn the cli writes its tool-call rows to the
//! agent's queue; we read them back with `agents read all` + `agents
//! read id` and dedupe the function names.
//!
//! The assertion: across all three turns, the deduplicated set of
//! tool-call names contains **at least 2 unique entries**. That proves
//! the mock's RNG-driven tool selection actually hit at least two of
//! the five duplicate-named upstreams — i.e., the proxy didn't collapse
//! the duplicates and the cli routed each call back to the correct
//! plugin instance.
//!
//! Skip-gate: `OBJECTIVEAI_TEST_PORT` must point at a running test API.

mod cli_test_util;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;
use std::time::{Duration, Instant};

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::{
    Request as MessageRequest, RequestMessage,
};
use objectiveai_sdk::cli::command::agents::read::all::{
    Request as ReadAllRequest, ResponseItem as ReadAllItem, ResponseQueueItem,
};
use objectiveai_sdk::cli::command::agents::read::id::{
    Request as ReadIdRequest, Response as ReadIdResponse,
};
use objectiveai_sdk::cli::command::agents::spawn::{
    AgentSpec, Request as SpawnRequest, RequestPrompt, ResponseItem as SpawnResponseItem,
};
use serde_json::{Value, json};

const PLUGIN_NAMES: [&str; 5] = [
    "dup-alpha",
    "dup-bravo",
    "dup-charlie",
    "dup-delta",
    "dup-echo",
];
/// Arbitrary seed. The mock's RNG is hash-seeded from prompt + tool
/// names + seed; if this value ever stops yielding ≥2 unique calls
/// across three turns, try another small integer.
const SEED: i64 = 7;

static BUILD_PLUGIN_ONCE: Once = Once::new();

fn plugin_binary() -> PathBuf {
    let target = cli_test_util::test_target_dir();
    let mut path = target.join("debug/test-mcp-plugin-named");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    BUILD_PLUGIN_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args([
                "build",
                "-p",
                "test-mcp-plugin-named",
                "--target-dir",
                target.to_str().unwrap(),
            ])
            .status()
            .expect("spawn cargo build test-mcp-plugin-named");
        assert!(status.success(), "test-mcp-plugin-named build failed");
    });
    path
}

/// Stage all five plugin installs at `<base>/plugins/<name>` with
/// manifests at `<base>/plugins/<name>.json`. The same binary backs
/// every install — uniqueness comes from the install name (which the
/// agent's `client_objectiveai_mcp.plugins[].name` references) plus
/// the `--name` argv we feed each one (which becomes its upstream
/// `serverInfo.name`).
fn stage_plugins(base: &Path) {
    let plugins = base.join("plugins");
    let bin = plugin_binary();
    for name in PLUGIN_NAMES {
        let install = plugins.join(name);
        std::fs::create_dir_all(&install).unwrap();
        let manifest = json!({
            "description": format!("{name} fixture"),
            "version": "1.0.0",
            "owner": "testorg",
            "mcp_servers": [
                { "name": "demo", "url": "http://127.0.0.1:0", "authorization": false }
            ]
        });
        std::fs::write(
            plugins.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let installed = install.join(if cfg!(windows) { "plugin.exe" } else { "plugin" });
        std::fs::copy(&bin, &installed).expect("copy fixture binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&installed, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}

/// Bare-bones plain mock agent. No `calls` override, no fallbacks,
/// no fancy modes. The plugin surface lists every staged plugin's
/// `demo` MCP; argv to each plugin is `--name <plugin install name>`
/// so each upstream's `serverInfo.name` matches its install name and
/// the agent-visible prefixed tool names are unique with no
/// disambiguation suffix.
fn mock_agent() -> Value {
    let plugins: Vec<Value> = PLUGIN_NAMES
        .iter()
        .map(|name| {
            json!({
                "owner": "testorg",
                "name": name,
                "version": "1.0.0",
                "executable": false,
                "mcp_servers": [{
                    "name": "demo",
                    "arguments": { "name": name }
                }]
            })
        })
        .collect();
    json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "client_objectiveai_mcp": { "plugins": plugins }
    })
}

/// Wait for the cli-stream child to have flushed an agent's response
/// continuation AND torn down its socket. Mirrors the polling pattern
/// from `plugin_mcp_dispatch_e2e::wait_for_completion`.
async fn wait_for_completion(base: &Path, spawn_id: &str) {
    let cont = base
        .join("logs/agents/completions/response/continuation")
        .join(format!("{spawn_id}.json"));
    let socket = base.join("pipes/cli").join(spawn_id).join("socket");
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if cont.exists() && !socket.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cli-stream did not flush continuation + tear down socket for {spawn_id} in 180s");
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_tool_names_routed_across_turns() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "skipping duplicate_tool_names_routed_across_turns: OBJECTIVEAI_TEST_PORT not set"
        );
        return;
    }
    let _ = cli_test_util::cli_binary();
    let _ = plugin_binary();

    let base = cli_test_util::test_base_dir();

    stage_plugins(&base);

    let agent = AgentSpec::Resolved(
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(mock_agent())
            .expect("mock agent must deserialize"),
    );
    let executor = cli_test_util::executor_with_base_dir(&base);

    // Turn 1: agents spawn ────────────────────────────────────────
    let spawn = SpawnRequest {
        prompt: RequestPrompt::Simple("use a tool".to_string()),
        agent,
        seed: Some(SEED),
        dangerous_advanced: None,
        jq: None,
    };
    let items: Vec<SpawnResponseItem> = cli_test_util::collect_stream(&executor, spawn).await;
    let spawn_id = items
        .iter()
        .find_map(|i| match i {
            SpawnResponseItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("agents spawn must emit a Chunk with agent_instance_hierarchy");
    wait_for_completion(&base, &spawn_id).await;

    // Turn 2: agents message — first continuation ─────────────────
    let msg1 = MessageRequest {
        agent_instance_hierarchy: spawn_id.clone(),
        message: RequestMessage::Simple("again".to_string()),
        seed: Some(SEED),
        jq: None,
    };
    let _ = executor
        .execute_one::<_, objectiveai_sdk::cli::command::agents::message::Response>(msg1, None)
        .await
        .expect("agents message turn 2 executor call");
    wait_for_completion(&base, &spawn_id).await;

    // Turn 3: agents message — second continuation ───────────────
    let msg2 = MessageRequest {
        agent_instance_hierarchy: spawn_id.clone(),
        message: RequestMessage::Simple("one more".to_string()),
        seed: Some(SEED),
        jq: None,
    };
    let _ = executor
        .execute_one::<_, objectiveai_sdk::cli::command::agents::message::Response>(msg2, None)
        .await
        .expect("agents message turn 3 executor call");
    wait_for_completion(&base, &spawn_id).await;

    // Collect every assistant turn's tool_call rows via `agents read
    // all`, then resolve each row id through `agents read id` to pull
    // the typed `AssistantToolCallDelta` whose `function.name` carries
    // the prefixed tool name.
    let read_all = ReadAllRequest {
        agent_instance_hierarchies: vec![spawn_id.clone()],
        jq: None,
    };
    let read_items: Vec<ReadAllItem> = cli_test_util::collect_stream(&executor, read_all).await;

    let mut tool_call_ids: Vec<i64> = Vec::new();
    for item in &read_items {
        for q in &item.items {
            if let ResponseQueueItem::AssistantResponse {
                tool_calls: Some(ids),
                ..
            } = q
            {
                tool_call_ids.extend(ids.iter().copied());
            }
        }
    }
    assert!(
        !tool_call_ids.is_empty(),
        "expected ≥1 tool-call row across the three turns; got none — \
         the mock didn't pick any tools (seed/mode mismatch?)"
    );

    use objectiveai_sdk::cli::command::CommandExecutor;
    let mut unique: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in tool_call_ids {
        let resp: ReadIdResponse = executor
            .execute_one(ReadIdRequest { id, jq: None }, None)
            .await
            .unwrap_or_else(|e| panic!("agents read id {id} failed: {e:?}"));
        // Tool-call rows always come back as
        // `AgentsCompletionsResponseMessagesToolCalls(AssistantToolCallDelta)`.
        // Other variants would mean the queue cross-referenced a
        // non-tool-call row id — surface that as a panic so we notice.
        let name = match resp {
            ReadIdResponse::AgentsCompletionsResponseMessagesToolCalls(delta) => {
                delta.function.and_then(|f| f.name)
            }
            other => panic!(
                "expected AgentsCompletionsResponseMessagesToolCalls for tool-call row id {id}, got {other:?}"
            ),
        };
        if let Some(n) = name {
            unique.insert(n);
        }
    }

    assert!(
        unique.len() >= 2,
        "expected ≥2 unique tool-call names across all three turns, got {unique:?}",
    );
}
