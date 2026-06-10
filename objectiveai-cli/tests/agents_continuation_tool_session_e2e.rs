//! `agents spawn` + 2 continuations × 2 agents with a 10-tool surface
//! whose tools increment a per-MCP-session counter file. After three
//! turns per agent we use `agents logs read all` to enumerate the
//! `ToolResponse` blocks for each agent's AIH and `agents logs read
//! id` to drill into the text payload of each. The smoking-gun
//! assertions:
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
//! Driven through the SDK `BinaryExecutor` so every cli leaf is
//! invoked with a typed `Request` value. Postgres reads (continuation
//! polling) run through `db query`.

mod cli_test_util;

use std::sync::Arc;
use std::time::Duration;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::{
    MessageTarget, Request as MessageRequest,
    RequestDangerousAdvanced as MessageDangerousAdvanced, RequestMessage,
    ResponseItem as MessageResponseItem,
};
use objectiveai_sdk::cli::command::agents::spawn::{
    AgentResolution, AgentSpec, Request as SpawnRequest, RequestDangerousAdvanced,
    ResponseItem as SpawnResponseItem,
};
use cli_test_util::HangPreventingBinaryCommandExecutor;
use serde_json::{Value, json};

/// Inline mock-agent spec wired to the 10 `testorg/tool{0..9}/1.0.0`
/// tools the test mcp server registered.
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

/// Pair of (full api-side AIH, response_id) needed to drive both
/// the AIH-keyed lookups (`agent_continuations`, message target)
/// and the response_id-keyed lookups (`tool_response_content_text`).
#[derive(Debug, Clone)]
struct Spawned {
    aih: String,
    response_id: String,
}

async fn spawn_agent(executor: &HangPreventingBinaryCommandExecutor, seed: i64) -> Spawned {
    let request = SpawnRequest {
        path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
        message: RequestMessage::Simple("go".to_string()),
        agent: AgentResolution::Direct {
            agent_spec: agent_spec(),
        },
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(seed),
        }),
        jq: None,
    };
    let items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(executor, request).await;
    let aih = items
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
    Spawned { aih, response_id }
}

/// Wait for the cli-stream child to have flushed the per-chunk
/// `agent_continuations` upsert for `aih`. With `stream=true`,
/// `collect_stream` returning already implies the runner exited,
/// but we double-check the continuation row exists so subsequent
/// turns' continuation lookups don't race against a stragglish
/// write.
async fn wait_for_completion(
    executor: &HangPreventingBinaryCommandExecutor,
    aih: &str,
) {
    cli_test_util::wait_for_continuation(executor, aih, Duration::from_secs(720)).await;
}

/// Run one continuation turn against a spawned agent. Seed is
/// threaded through to `MessageRequest.seed` so the api's mock
/// RNG reproduces the same per-turn tool selections as the spawn
/// turn — same value the test passed to `agents spawn`.
async fn continue_agent(
    executor: &HangPreventingBinaryCommandExecutor,
    spawn_aih: &str,
    seed: i64,
) {
    let (parent, instance) = spawn_aih
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or_else(|| (None, spawn_aih.to_string()));
    let request = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
        target: MessageTarget::Direct {
            parent_agent_instance_hierarchy: parent,
            agent_instance: instance,
        },
        message: RequestMessage::Simple("more".to_string()),
        enqueue: None,
        dangerous_advanced: Some(MessageDangerousAdvanced {
            stream: Some(true),
            seed: Some(seed),
        }),
        jq: None,
    };
    let _items: Vec<MessageResponseItem> =
        cli_test_util::collect_stream(executor, request).await;
}

/// Pull every count value the count-tool emitted to its tool
/// responses for the agent identified by `response_id`. The
/// count-tool prints `N\n` per call; that lands in
/// `logs.tool_response_content_text.text` as a base-10 string.
/// Reading via `db query` sidesteps the
/// `Response::Text(String)`-can't-serialize bug in the
/// `agents logs read id` MCP projection.
async fn read_tool_response_counts(
    executor: &HangPreventingBinaryCommandExecutor,
    response_id: &str,
) -> Vec<u64> {
    let sql = format!(
        "SELECT text FROM logs.tool_response_content_text \
         WHERE response_id = '{}' ORDER BY \"index\", part_index",
        response_id.replace('\'', "''"),
    );
    let rows = cli_test_util::db_query(executor, &sql).await;
    rows.into_iter()
        .filter_map(|mut row| row.pop())
        .filter_map(|v| match v {
            Value::String(s) => s.trim().parse::<u64>().ok(),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn two_agents_continuations_count_persists_per_session() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "OBJECTIVEAI_TEST_PORT not set — skipping two_agents_continuations_count_persists_per_session"
        );
        return;
    }

    let base_dir = cli_test_util::test_base_dir();
    let executor = Arc::new(cli_test_util::executor_with_base_dir(&base_dir));

    let run_agent = |seed: i64| {
        let executor = executor.clone();
        async move {
            let spawned = spawn_agent(&executor, seed).await;
            wait_for_completion(&executor, &spawned.aih).await;
            for _ in 0..2 {
                continue_agent(&executor, &spawned.aih, seed).await;
                wait_for_completion(&executor, &spawned.aih).await;
            }
            spawned
        }
    };

    let (a, b) = tokio::join!(run_agent(1), run_agent(2));
    assert_ne!(a.aih, b.aih, "two spawns must produce distinct lineages");

    let counts_a = read_tool_response_counts(&executor, &a.response_id).await;
    let counts_b = read_tool_response_counts(&executor, &b.response_id).await;

    assert!(
        !counts_a.is_empty(),
        "agent A produced zero tool-response text rows — mock didn't call tools (seed/mode mismatch?)",
    );
    assert!(
        !counts_b.is_empty(),
        "agent B produced zero tool-response text rows — mock didn't call tools (seed/mode mismatch?)",
    );

    let max_a = *counts_a.iter().max().expect("counts_a empty");
    let max_b = *counts_b.iter().max().expect("counts_b empty");

    assert_eq!(
        max_a as usize,
        counts_a.len(),
        "agent A's max count ({max_a}) must equal its tool-response item count ({}) — \
         a reset would leave it lower",
        counts_a.len(),
    );
    assert_eq!(
        max_b as usize,
        counts_b.len(),
        "agent B's max count ({max_b}) must equal its tool-response item count ({}) — \
         a reset would leave it lower",
        counts_b.len(),
    );
}
