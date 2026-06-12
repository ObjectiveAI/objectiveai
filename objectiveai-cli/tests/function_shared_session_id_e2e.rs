//! Five identical mock agents in one vector.completion task all dial
//! the SAME in-process axum MCP server. The test runs the function
//! execution once (5 agents → 5 tool calls in turn 1, "done" in
//! turn 2), then sends a fresh user message to each one in parallel —
//! the cli handles continuation transparently per agent.
//!
//! On every initialize the server mints a fresh `Mcp-Session-Id` and
//! tags it as either `new` (no inbound `Mcp-Session-Id` header — the
//! proxy is dialing fresh) or `resumed` (header present — the proxy
//! is replaying a prior session id). Every `tools/call` looks up the
//! inbound session's `is_new` flag and appends
//! `"{is_new}-{response_id}"` to a file under the test's state dir.
//!
//! Assertion: exactly 10 unique lines, with at least 5 starting
//! `true-` (5 fresh inits) and at least 5 starting `false-` (5
//! resumptions). The output file is a side-effect of the test's own
//! axum MCP server — not a cli log file — so it stays a filesystem
//! read.

mod cli_test_util;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, post},
};
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::{
    Request as MessageRequest,
    RequestDangerousAdvanced as MessageDangerousAdvanced, RequestMessage,
    Response as MessageResponse,
};
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn::{
    Request as SpawnRequest,
    RequestDangerousAdvanced as SpawnDangerousAdvanced,
    ResponseItem as SpawnResponseItem,
};
use serde_json::{Value, json};
use tokio::sync::Mutex;

const SERVER_NAME: &str = "srv";
const TOOL_NAME: &str = "ping";

#[derive(Clone)]
struct ServerState {
    output_path: Arc<PathBuf>,
    is_new_by_session: Arc<Mutex<HashMap<String, bool>>>,
}

async fn handle_post(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let method = body
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let id = body.get("id").cloned();
    match method.as_str() {
        "initialize" => {
            let is_new = headers.get("Mcp-Session-Id").is_none();
            let server_sid = format!("srv-sid-{}", uuid::Uuid::new_v4());
            state
                .is_new_by_session
                .lock()
                .await
                .insert(server_sid.clone(), is_new);
            let mut resp = Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": SERVER_NAME, "version": "0.0.0" }
                }
            }))
            .into_response();
            resp.headers_mut().insert(
                "Mcp-Session-Id",
                HeaderValue::from_str(&server_sid).unwrap(),
            );
            resp
        }
        "notifications/initialized" => StatusCode::ACCEPTED.into_response(),
        "tools/list" => Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [{
                    "name": TOOL_NAME,
                    "description": "no-op",
                    "inputSchema": { "type": "object", "additionalProperties": true }
                }]
            }
        }))
        .into_response(),
        "tools/call" => {
            let inbound_sid = headers
                .get("Mcp-Session-Id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let is_new = state
                .is_new_by_session
                .lock()
                .await
                .get(&inbound_sid)
                .copied();
            let label = match is_new {
                Some(true) => "true",
                Some(false) => "false",
                None => "unknown",
            };
            let rid = headers
                .get("X-OBJECTIVEAI-RESPONSE-ID")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(state.output_path.as_ref())
                .expect("open response-ids file");
            writeln!(f, "{label}-{rid}").expect("write line");
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{ "type": "text", "text": "ok" }],
                    "isError": false
                }
            }))
            .into_response()
        }
        other => (StatusCode::NOT_FOUND, format!("unknown {other}")).into_response(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn shared_mcp_session_preserves_per_agent_identity_with_resumption() {
    let base = cli_test_util::test_base_dir();

    let output_path = Arc::new(base.join("response-ids.txt"));

    let state = ServerState {
        output_path: output_path.clone(),
        is_new_by_session: Arc::new(Mutex::new(HashMap::new())),
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind axum");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route("/", post(handle_post))
        .route("/", delete(|| async { StatusCode::OK }))
        .with_state(state);
    let _server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let prefixed_tool = format!("{SERVER_NAME}_{TOOL_NAME}");
    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "mcp_servers": [{ "url": url, "authorization": false }],
        "calls": [
            { "tool_calls": [{ "name": prefixed_tool, "arguments": "{}" }], "content": "" },
            { "tool_calls": [], "content": "done" },
            { "tool_calls": [{ "name": prefixed_tool, "arguments": "{}" }], "content": "" },
            { "tool_calls": [], "content": "done2" }
        ]
    });

    let executor = cli_test_util::executor().await;

    let spawn_agent = |seed: i64| {
        let executor = &executor;
        let agent_json = agent_json.clone();
        async move {
            let agent = AgentSelector::Ref {
                agent: AgentRef::Resolved(
                    serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                        agent_json,
                    )
                    .expect("inline mock agent must deserialize"),
                ),
            };
            let request = SpawnRequest {
                path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
                message: RequestMessage::Simple("go".to_string()),
                agent,
                dangerous_advanced: Some(SpawnDangerousAdvanced {
                    stream: Some(true),
                    seed: Some(seed),
                    skip_lock: None,
                }),
                jq: None,
            };
            let items: Vec<SpawnResponseItem> =
                cli_test_util::collect_stream(executor, request).await;
            items
                .iter()
                .find_map(|item| match item {
                    SpawnResponseItem::Chunk(chunk)
                        if !chunk.agent_instance_hierarchy.is_empty() =>
                    {
                        Some(chunk.agent_instance_hierarchy.clone())
                    }
                    _ => None,
                })
                .expect(
                    "agents spawn must emit a Chunk with a non-empty agent_instance_hierarchy",
                )
        }
    };

    // Run 1: spawn 5 agents sequentially. Each spawn finishes turn 1
    // (tool_call + "done") before the next starts — guarantees the
    // per-agent continuation row exists before any continuation
    // turn races against it.
    let mut aihs: Vec<(String, i64)> = Vec::with_capacity(5);
    for i in 0..5 {
        let seed = i + 1;
        let aih = spawn_agent(seed).await;
        cli_test_util::wait_for_continuation(&executor, &aih, Duration::from_secs(180)).await;
        aihs.push((aih, seed));
    }

    // Send `agents message` to all 5 in parallel. Split each AIH
    // into (parent, instance) so the message handler reconstructs
    // the same full lineage when composing the lock-file key.
    let send_futures = aihs.iter().map(|(aih, seed)| {
        let executor = &executor;
        let seed = *seed;
        let (parent, instance) = aih
            .rsplit_once('/')
            .map(|(p, i)| (Some(p.to_string()), i.to_string()))
            .unwrap_or((None, aih.clone()));
        let request = MessageRequest {
            path_type: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
            agent: AgentSelector::Instance {
                parent_agent_instance_hierarchy: parent,
                agent_instance: instance,
            },
            message: RequestMessage::Simple("again".to_string()),
            dangerous_advanced: Some(MessageDangerousAdvanced { seed: Some(seed) }),
            jq: None,
        };
        async move {
            let _resp: MessageResponse =
                cli_test_util::execute_one(executor, request).await;
        }
    });
    futures::future::join_all(send_futures).await;

    // Wait for each agent's `agent_continuations` row to reflect
    // the post-continuation continuation.
    let wait_futures = aihs.iter().map(|(aih, _seed)| {
        let executor = &executor;
        let aih = aih.clone();
        async move {
            cli_test_util::wait_for_continuation(executor, &aih, Duration::from_secs(180)).await;
        }
    });
    futures::future::join_all(wait_futures).await;

    let raw = std::fs::read_to_string(output_path.as_ref()).unwrap_or_default();
    let lines: Vec<String> = raw
        .lines()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect();
    let unique: HashSet<&String> = lines.iter().collect();
    let trues = lines.iter().filter(|l| l.starts_with("true-")).count();
    let falses = lines.iter().filter(|l| l.starts_with("false-")).count();
    let unknowns = lines.iter().filter(|l| l.starts_with("unknown-")).count();

    assert_eq!(
        unique.len(),
        10,
        "expected 10 unique lines across 5 agent spawns + 5 continuation messages, \
         got {} unique from {} total lines (true={trues}, false={falses}, unknown={unknowns}): {lines:?}",
        unique.len(),
        lines.len(),
    );
    assert_eq!(
        unknowns, 0,
        "no line should be `unknown-...` (MCP-side missed initialize), got {unknowns} from {lines:?}",
    );
    assert_eq!(
        trues + falses,
        10,
        "every line must be `true-...` (fresh session) or `false-...` (resumption), got true={trues} false={falses} unknown={unknowns} from {lines:?}",
    );
}
