//! Five plugin installs, each pointing at the same `test-mcp-plugin-named`
//! fixture binary launched with an **identical** `--name` argv. Every
//! upstream's `serverInfo.name` therefore collides on the same string,
//! so the proxy must disambiguate the five colliding upstreams itself
//! to surface five distinct agent-visible tool names.
//!
//! A bare-bones plain mock agent runs three CLI turns against the
//! same `agent_instance_hierarchy`: one `agents spawn`, two
//! `agents message`. After every turn the cli writes its response
//! blob to `objectiveai.agent_completion_responses`; we extract every
//! tool-call's `function.name` from
//! `body.messages[*].tool_calls[*].function.name` via `db query` and
//! dedupe.
//!
//! The assertion: across all three turns, the deduplicated set of
//! tool-call names contains **at least 2 unique entries**. That
//! proves the mock's RNG-driven tool selection hit at least two of
//! the five colliding-server-name upstreams — i.e., the proxy
//! produced ≥2 distinct surfaced names instead of collapsing the
//! five into one.

mod cli_test_util;

use std::time::Duration;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::logs::read::all::{
    AssistantResponsePartType, Request as ReadAllRequest, ResponseItem as ReadAllItem,
    Target as ReadAllTarget,
};
use objectiveai_sdk::cli::command::agents::message::{
    Request as MessageRequest,
    RequestDangerousAdvanced as MessageDangerousAdvanced, RequestMessage,
    Response as MessageResponse,
};
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn::{
    Request as SpawnRequest, RequestDangerousAdvanced,
    ResponseItem as SpawnResponseItem,
};
use serde_json::{Value, json};

const PLUGIN_INSTALL_NAMES: [&str; 5] = [
    "same-alpha",
    "same-bravo",
    "same-charlie",
    "same-delta",
    "same-echo",
];

/// Shared `--name` argv passed to every plugin binary so every
/// upstream's `serverInfo.name` echoes the same string.
const SHARED_SERVER_NAME: &str = "same";

const SEED: i64 = 42;

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

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_server_names_routed_across_turns() {
    let agent = AgentSelector::Ref {
        agent: AgentRef::Resolved(
            serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                mock_agent(),
            )
            .expect("mock agent must deserialize"),
        ),
    };
    let executor = cli_test_util::executor().await;

    // Turn 1: agents spawn
    let spawn = SpawnRequest {
        path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
        message: RequestMessage::Simple("use a tool".to_string()),
        agent,
        dangerous_advanced: Some(RequestDangerousAdvanced {
            stream: Some(true),
            seed: Some(SEED),
            skip_lock: None,
        }),
        base: Default::default(),
    };
    let items: Vec<SpawnResponseItem> = cli_test_util::collect_stream(&executor, spawn).await;
    let spawn_aih = items
        .iter()
        .find_map(|i| match i {
            SpawnResponseItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("agents spawn must emit a Chunk with a non-empty agent_instance_hierarchy");
    let target_instance = spawn_aih
        .rsplit_once('/')
        .map(|(_, i)| i.to_string())
        .unwrap_or_else(|| spawn_aih.clone());
    let target_parent = spawn_aih
        .rsplit_once('/')
        .map(|(p, _)| Some(p.to_string()))
        .unwrap_or(None);
    cli_test_util::wait_for_agent(&executor, &spawn_aih).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Turn 2: agents message — first continuation
    let msg1 = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
        agent: AgentSelector::Instance {
            parent_agent_instance_hierarchy: target_parent.clone(),
            agent_instance: target_instance.clone(),
        },
        message: RequestMessage::Simple("again".to_string()),
        dangerous_advanced: Some(MessageDangerousAdvanced { seed: Some(SEED) }),
        base: Default::default(),
    };
    let _resp: MessageResponse = cli_test_util::execute_one(&executor, msg1).await;
    cli_test_util::wait_for_agent(&executor, &spawn_aih).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Turn 3: agents message — second continuation
    let msg2 = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
        agent: AgentSelector::Instance {
            parent_agent_instance_hierarchy: target_parent.clone(),
            agent_instance: target_instance.clone(),
        },
        message: RequestMessage::Simple("one more".to_string()),
        dangerous_advanced: Some(MessageDangerousAdvanced { seed: Some(SEED) }),
        base: Default::default(),
    };
    let _resp: MessageResponse = cli_test_util::execute_one(&executor, msg2).await;
    cli_test_util::wait_for_agent(&executor, &spawn_aih).await;

    // Walk every assistant block from `agents logs read all`,
    // filter to `ToolCall` parts, collect `function_name`.
    let read_all = ReadAllRequest {
        path_type: objectiveai_sdk::cli::command::agents::logs::read::all::Path::AgentsLogsReadAll,
        targets: vec![ReadAllTarget::Direct {
            parent_agent_instance_hierarchy: None,
            agent_instance: target_instance.clone(),
        }],
        after_id: None,
        limit: None,
        base: Default::default(),
    };
    let blocks: Vec<ReadAllItem> = cli_test_util::collect_stream(&executor, read_all).await;
    let mut names: Vec<String> = Vec::new();
    for block in &blocks {
        if let ReadAllItem::AssistantResponse { parts, .. } = block {
            for part in parts {
                if matches!(part.r#type, AssistantResponsePartType::ToolCall) {
                    names.push(
                        part.function_name
                            .clone()
                            .expect("ToolCall part must carry function_name"),
                    );
                }
            }
        }
    }
    assert!(
        !names.is_empty(),
        "expected ≥1 tool-call row across the three turns; got none — \
         the mock didn't pick any tools (seed/mode mismatch?)"
    );
    let unique: std::collections::HashSet<String> = names.into_iter().collect();
    assert!(
        unique.len() >= 2,
        "expected ≥2 unique tool-call names across all three turns, got {unique:?} — \
         the proxy may be collapsing colliding-server-name upstreams instead of \
         disambiguating them",
    );
}
