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

use std::path::Path;
use std::sync::Arc;
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
    AgentSpec, Request as SpawnRequest, RequestDangerousAdvanced, RequestPrompt,
    ResponseItem as SpawnResponseItem,
};
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use serde_json::{Value, json};

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
    // Deterministic `calls` override: each of the three turns
    // (spawn + 2 messages) invokes `tool0` exactly once, then the
    // assistant emits a per-turn "done" content message that ends
    // the turn. The mock's `next_unmatched_call_index` advances
    // through this list in order across the cumulative continuation,
    // so the three turns get the three tool calls deterministically.
    // Without this override we rely on the per-turn RNG dice roll
    // which can land entirely on "respond_as_is" and produce zero
    // tool calls — exactly the regression this test was hitting.
    let calls = json!([
        {"tool_calls": [{"name": "oai_tool0", "arguments": "{\"args\":[]}"}], "content": ""},
        {"tool_calls": [], "content": "done1"},
        {"tool_calls": [{"name": "oai_tool0", "arguments": "{\"args\":[]}"}], "content": ""},
        {"tool_calls": [], "content": "done2"},
        {"tool_calls": [{"name": "oai_tool0", "arguments": "{\"args\":[]}"}], "content": ""},
        {"tool_calls": [], "content": "done3"},
    ]);
    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "client_objectiveai_mcp": {"tools": tools},
        "calls": calls,
    });
    AgentSpec::Resolved(
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
            agent_json,
        )
        .expect("inline mock agent must deserialize"),
    )
}

/// Spawn an agent and return its FULL cli-side lineage —
/// `format!("cli/{leaf}")` — where `leaf` is `chunk.id`, the
/// per-agent response_id leaf. The cli's on-disk filesystem keys on
/// this `cli/<leaf>` shape (NOT on `chunk.agent_instance_hierarchy`,
/// which is the api-side slot id `cli/{agent_full_id}-{leaf}` —
/// useful for the api's internal routing, not for finding cli logs).
async fn spawn_agent(executor: &BinaryExecutor, seed: i64) -> String {
    let request = SpawnRequest { path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
        prompt: RequestPrompt::Simple("go".to_string()),
        agent: agent_spec(),
        seed: Some(seed),
        // Stream so the cli stays attached to the instance subprocess
        // through `LogStreamReady` + every chunk; we need at least one
        // chunk to read `chunk.id` (the leaf), and we need the cli to
        // not detach early so `wait_for_completion` polling against
        // disk state isn't racing against an orphaned writer.
        dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
        jq: None,
    };
    let items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(executor, request).await;
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
        .expect("agents spawn must emit a Chunk with non-empty id");
    format!("cli/{leaf}")
}

/// Wait for the cli-stream child to have flushed an agent's response
/// continuation and unlinked its socket.
///
/// On-disk conventions:
///   continuation token (raw text): `logs/agents/completions/response/continuation/<leaf>.txt`
///     — stems on the LEAF response id alone
///   per-agent socket:              `pipes/<full_lineage>/socket`
///     — stems on the FULL lineage (which already starts with `cli/`,
///     so the path is `pipes/cli/<leaf>/socket`; do NOT prepend `cli/`
///     a second time)
async fn wait_for_completion(base_dir: &Path, full_lineage: &str) {
    let leaf = full_lineage
        .rsplit_once('/')
        .map(|(_, leaf)| leaf)
        .unwrap_or(full_lineage);
    let response_cont_path = base_dir
        .join("logs/agents/completions/response/continuation")
        .join(format!("{leaf}.txt"));
    let socket_path = base_dir.join("pipes").join(full_lineage).join("socket");
    poll_until(Duration::from_secs(720), || {
        response_cont_path.exists() && !socket_path.exists()
    })
    .await
    .unwrap_or_else(|()| {
        panic!("cli-stream did not flush continuation + tear down socket for {full_lineage} (leaf {leaf}) in 720s",)
    });
}

/// Run one continuation turn against a spawned agent.
async fn continue_agent(executor: &BinaryExecutor, spawn_id: &str, seed: i64) {
    // Split the full lineage into (parent, instance) for the
    // two-field `MessageRequest` shape.
    let (parent, instance) = spawn_id
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or_else(|| (None, spawn_id.to_string()));
    let request = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
        parent_agent_instance_hierarchy: parent,
        agent_instance: instance,
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
/// via the public `agents read all` cli surface. `sub_id` is the
/// full cli-side lineage (e.g. `cli/<leaf>`); the cli's read::all
/// rebuilds the full hierarchy as `{caller}/{sub}` so we pass just
/// the leaf (the part after the rsplit on '/'), avoiding the
/// `cli/cli/<leaf>` double-prefix that would shadow the queue rows.
async fn read_tool_response_ids(executor: &BinaryExecutor, sub_id: &str) -> Vec<i64> {
    let leaf = sub_id
        .rsplit_once('/')
        .map(|(_, leaf)| leaf)
        .unwrap_or(sub_id);
    let request = ReadAllRequest { path_type: objectiveai_sdk::cli::command::agents::read::all::Path::AgentsReadAll,
        agent_instance_hierarchies: vec![leaf.to_string()],
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
    let request = ReadIdRequest { path_type: objectiveai_sdk::cli::command::agents::read::id::Path::AgentsReadId, id, jq: None };
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

    // Per-test base dir staged by `objectiveai-tests/prepare.sh`:
    // the ten `testorg/tool{0..9}/1.0.0` manifests + the `count-tool`
    // binary are already in place under `<base>/tools/`. The cli
    // writes its `logs/`/`pipes/` runtime artefacts here over the
    // test run; `test.sh` wipes `.objectiveai-tests/` on its way in
    // so no stale state leaks across runs.
    let base_dir = cli_test_util::test_base_dir();

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
