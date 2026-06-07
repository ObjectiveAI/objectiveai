//! Five plugin installs, each pointing at the same `test-mcp-plugin-named`
//! fixture binary launched with an **identical** `--name` argv. Every
//! upstream's `serverInfo.name` therefore collides on the same string,
//! so the proxy must disambiguate the five colliding upstreams itself
//! to surface five distinct agent-visible tool names. Each upstream
//! still advertises a single inner tool called `invoke`.
//!
//! Companion to `agents_duplicate_tool_names_e2e`, which exercises the
//! easier case where each upstream's `serverInfo.name` is already
//! distinct — there the proxy only has to apply its standard
//! `<server>_<tool>` prefix scheme. Here the prefix alone is not
//! enough: all five would map to the same `<server>_invoke`, and the
//! proxy has to add a disambiguation suffix (or equivalent) to keep
//! them addressable.
//!
//! A bare-bones plain mock agent runs three CLI turns against the
//! same `agent_instance_hierarchy`: one `agents instances spawn`, two
//! `agents instances message`. After every turn the cli writes its
//! tool-call rows to the agent's queue; we read them back with
//! `agents instances read all` + `agents instances read id` and
//! dedupe the function names.
//!
//! The assertion: across all three turns, the deduplicated set of
//! tool-call names contains **at least 2 unique entries**. That
//! proves the mock's RNG-driven tool selection hit at least two of
//! the five colliding-server-name upstreams — i.e., the proxy
//! produced ≥2 distinct surfaced names instead of collapsing the
//! five into one.
//!
//! Skip-gate: `OBJECTIVEAI_TEST_PORT` must point at a running test API.

mod cli_test_util;

use std::path::Path;
use std::time::{Duration, Instant};

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::instances::message::{
    MessageTarget, Request as MessageRequest,
    RequestDangerousAdvanced as MessageDangerousAdvanced, RequestMessage,
    ResponseItem as MessageResponseItem,
};
use objectiveai_sdk::cli::command::agents::instances::read::all::{
    Request as ReadAllRequest, ResponseItem as ReadAllItem, ResponseQueueItem,
    Target as ReadAllTarget,
};
use objectiveai_sdk::cli::command::agents::instances::read::id::{
    Request as ReadIdRequest, Response as ReadIdResponse,
};
use objectiveai_sdk::cli::command::agents::instances::spawn::{
    AgentSpec, Request as SpawnRequest, RequestDangerousAdvanced, RequestPrompt,
    ResponseItem as SpawnResponseItem,
};
use serde_json::{Value, json};

/// Five distinct plugin install names so `prepare.sh` slots five
/// separate binary copies on disk; the cli still treats them as five
/// independent plugin entries in `client_objectiveai_mcp.plugins`.
const PLUGIN_INSTALL_NAMES: [&str; 5] = [
    "same-alpha",
    "same-bravo",
    "same-charlie",
    "same-delta",
    "same-echo",
];

/// The single shared `--name` argv passed to every plugin binary, so
/// every upstream's `serverInfo.name` echoes the same string. This is
/// what forces the proxy to disambiguate the colliding upstreams
/// rather than relying on naturally-unique server names.
const SHARED_SERVER_NAME: &str = "same";

/// Arbitrary seed. The mock's RNG is hash-seeded from prompt + tool
/// names + seed; if this value ever stops yielding ≥2 unique calls
/// across three turns, try another small integer.
const SEED: i64 = 42;

/// Bare-bones plain mock agent. No `calls` override, no fallbacks,
/// no fancy modes. The plugin surface lists every staged plugin's
/// `demo` MCP; argv to each plugin is `--name same` so all five
/// upstreams claim the same `serverInfo.name` and the proxy is the
/// one responsible for keeping them addressable.
fn mock_agent() -> Value {
    let plugins: Vec<Value> = PLUGIN_INSTALL_NAMES
        .iter()
        .map(|name| {
            json!({
                "owner": "testorg",
                "name": name,
                "version": "1.0.0",
                "executable": false,
                "mcp_servers": [{
                    "name": "demo",
                    "arguments": { "name": SHARED_SERVER_NAME }
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
/// continuation AND torn down its socket.
///
/// On-disk conventions:
///   continuation token (raw text): `logs/agents/completions/response/continuation/<leaf>.txt`
///     — stems on the LEAF response id alone
///   per-agent socket:              `pipes/<full_lineage>/socket`
///     — stems on the FULL lineage (which already starts with `cli/`)
async fn wait_for_completion(base: &Path, full_lineage: &str) {
    let leaf = full_lineage
        .rsplit_once('/')
        .map(|(_, leaf)| leaf)
        .unwrap_or(full_lineage);
    let cont = base
        .join("logs/agents/completions/response/continuation")
        .join(format!("{leaf}.txt"));
    let socket = base.join("pipes").join(full_lineage).join("socket");
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if cont.exists() && !socket.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!(
        "cli-stream did not flush continuation + tear down socket for {full_lineage} (leaf {leaf}) in 180s"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_server_names_routed_across_turns() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "skipping duplicate_server_names_routed_across_turns: OBJECTIVEAI_TEST_PORT not set"
        );
        return;
    }
    let base = cli_test_util::test_base_dir();

    let agent = AgentSpec::Resolved(
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(mock_agent())
            .expect("mock agent must deserialize"),
    );
    let executor = cli_test_util::executor_with_base_dir(&base);

    // Turn 1: agents instances spawn ──────────────────────────────
    let spawn = SpawnRequest { path_type: objectiveai_sdk::cli::command::agents::instances::spawn::Path::AgentsInstancesSpawn,
        prompt: RequestPrompt::Simple("use a tool".to_string()),
        agent,
        agent_tag: None,
        seed: Some(SEED),
        // Stream so we get `Chunk(_)` items (needed for `chunk.id`)
        // and so the cli stays attached to the instance subprocess
        // through completion (otherwise `wait_for_completion` races
        // against the orphaned writer).
        dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
        jq: None,
    };
    let items: Vec<SpawnResponseItem> = cli_test_util::collect_stream(&executor, spawn).await;
    let leaf = items
        .iter()
        .find_map(|i| match i {
            SpawnResponseItem::Chunk(c) if !c.id.is_empty() => Some(c.id.clone()),
            _ => None,
        })
        .expect("agents instances spawn must emit a Chunk with non-empty id");
    let spawn_id = format!("cli/{leaf}");
    wait_for_completion(&base, &spawn_id).await;

    let (parent, instance) = spawn_id
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or_else(|| (None, spawn_id.clone()));

    // Settle delay — same rationale as agents_duplicate_tool_names_e2e:
    // let the api fully release per-instance proxy connections + reverse
    // channels before re-attaching with a new instance subprocess.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Turn 2: agents instances message — first continuation ──────
    let msg1 = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::instances::message::Path::AgentsInstancesMessage,
        target: MessageTarget::Direct {
            parent_agent_instance_hierarchy: parent.clone(),
            agent_instance: instance.clone(),
            agent_tag: None,
        },
        message: RequestMessage::Simple("again".to_string()),
        seed: Some(SEED),
        dangerous_advanced: Some(MessageDangerousAdvanced {
            stream: Some(true),
        }),
        jq: None,
    };
    let _items: Vec<MessageResponseItem> =
        cli_test_util::collect_stream(&executor, msg1).await;
    wait_for_completion(&base, &spawn_id).await;

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Turn 3: agents instances message — second continuation ─────
    let msg2 = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::instances::message::Path::AgentsInstancesMessage,
        target: MessageTarget::Direct {
            parent_agent_instance_hierarchy: parent.clone(),
            agent_instance: instance.clone(),
            agent_tag: None,
        },
        message: RequestMessage::Simple("one more".to_string()),
        seed: Some(SEED),
        dangerous_advanced: Some(MessageDangerousAdvanced {
            stream: Some(true),
        }),
        jq: None,
    };
    let _items: Vec<MessageResponseItem> =
        cli_test_util::collect_stream(&executor, msg2).await;
    wait_for_completion(&base, &spawn_id).await;

    // Collect every assistant turn's tool_call rows via `agents
    // instances read all`, then resolve each row id through `agents
    // instances read id` to pull the typed `AssistantToolCallDelta`
    // whose `function.name` carries the (proxy-disambiguated)
    // prefixed tool name.
    let (read_parent, read_instance) = spawn_id
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or_else(|| (None, spawn_id.clone()));
    let read_all = ReadAllRequest { path_type: objectiveai_sdk::cli::command::agents::instances::read::all::Path::AgentsInstancesReadAll,
        targets: vec![ReadAllTarget::Direct {
            parent_agent_instance_hierarchy: read_parent,
            agent_instance: read_instance,
        }],
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
            .execute_one(ReadIdRequest { path_type: objectiveai_sdk::cli::command::agents::instances::read::id::Path::AgentsInstancesReadId, id, jq: None }, None)
            .await
            .unwrap_or_else(|e| panic!("agents instances read id {id} failed: {e:?}"));
        let name = match resp {
            ReadIdResponse::AgentsCompletionsResponseMessagesAssistantToolCalls(delta) => {
                delta.function.and_then(|f| f.name)
            }
            other => panic!(
                "expected AgentsCompletionsResponseMessagesAssistantToolCalls for tool-call row id {id}, got {other:?}"
            ),
        };
        if let Some(n) = name {
            unique.insert(n);
        }
    }

    assert!(
        unique.len() >= 2,
        "expected ≥2 unique tool-call names across all three turns, got {unique:?} — \
         the proxy may be collapsing colliding-server-name upstreams instead of \
         disambiguating them",
    );
}
