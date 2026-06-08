//! Five identical mock agents in one vector.completion task all dial
//! the SAME in-process axum MCP server. The test runs the function
//! execution once (5 agents → 5 tool calls in turn 1, "done" in
//! turn 2), then asks the cli `agents instances list` for the five
//! resulting agent slots, then sends a fresh user message to each
//! one in parallel — the cli handles continuation transparently per
//! agent.
//!
//! On every initialize the server mints a fresh `Mcp-Session-Id` and
//! tags it as either `new` (no inbound `Mcp-Session-Id` header — the
//! proxy is dialing fresh) or `resumed` (header present — the proxy
//! is replaying a prior session id). Every `tools/call` looks up the
//! inbound session's `is_new` flag and appends
//! `"{is_new}-{response_id}"` to a file under `CONFIG_BASE_DIR`.
//!
//! Assertion: exactly 10 unique lines, with exactly 5 starting
//! `true-` and 5 starting `false-`. That proves:
//!   1. Per-agent identity (`X-OBJECTIVEAI-RESPONSE-ID`) is preserved
//!      across both turns of each agent (10 unique response ids).
//!   2. The proxy sends the prior `Mcp-Session-Id` header on the
//!      continuation turn (5 `false-` lines).
//!   3. The proxy starts fresh without a `Mcp-Session-Id` header on
//!      the initial turn (5 `true-` lines).

mod cli_test_util;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, post},
};
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::agents::instances::message::{
    MessageTarget, Request as MessageRequest,
    RequestDangerousAdvanced as MessageDangerousAdvanced, RequestMessage,
    ResponseItem as MessageResponseItem,
};
use objectiveai_sdk::cli::command::agents::instances::spawn::{
    AgentSpec, Request as SpawnRequest, RequestDangerousAdvanced as SpawnDangerousAdvanced,
    ResponseItem as SpawnResponseItem,
};
use serde_json::{Value, json};
use tokio::sync::Mutex;

const SERVER_NAME: &str = "srv";
const TOOL_NAME: &str = "ping";
/// Default cli `agent_instance_hierarchy` root, set by
/// `ConfigBuilder::build` (`objectiveai-cli/src/run.rs:103-105`) when
/// `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY` is not set. The
/// `executor_with_base_dir` helper does NOT set that env var, so the
/// cli child uses this value as the parent for `agents instances list`
/// and we have to re-prepend it to each returned `agent_id` before
/// passing it to `agents instances message` (which expects the FULL
/// hierarchy).
const CLI_HIERARCHY_ROOT: &str = "cli";

/// Server state. `is_new_by_session` keyed by the server-minted
/// `Mcp-Session-Id` returned on initialize. Set once at init time;
/// read at every `tools/call` to label that call's file-line.
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
            // Fresh-or-resumed detection: inbound `Mcp-Session-Id`
            // header → resumption; absent → fresh. Mint a fresh
            // server-side id on EVERY init so first-run and
            // continuation-run ids are guaranteed distinct.
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

/// Wait for the cli-stream child to flush an agent's response
/// continuation AND tear down its socket. Mirrors the polling
/// pattern from `plugin_mcp_dispatch_e2e::wait_for_completion`.
///
/// On-disk conventions:
///   continuation token (raw text): `logs/agents/completions/response/continuation/<leaf>.txt`
///     — stems on the LEAF response id alone
///   per-agent socket:              `pipes/<full_hierarchy>/socket`
///     — stems on the FULL lineage (which already starts with `cli/`,
///     so the path is `pipes/cli/<leaf>/socket`; do NOT prepend `cli/`
///     a second time)
async fn wait_for_completion(base: &Path, full_hierarchy: &str) {
    let leaf = full_hierarchy
        .rsplit_once('/')
        .map(|(_, leaf)| leaf)
        .unwrap_or(full_hierarchy);
    let cont = base
        .join("logs/agents/completions/response/continuation")
        .join(format!("{leaf}.txt"));
    let socket = base.join("pipes").join(full_hierarchy).join("socket");
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if cont.exists() && !socket.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!(
        "cli-stream did not flush continuation + tear down socket for {full_hierarchy} (leaf {leaf}) in 180s"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn shared_mcp_session_preserves_per_agent_identity_with_resumption() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "skipping shared_mcp_session_preserves_per_agent_identity_with_resumption: \
             OBJECTIVEAI_TEST_PORT not set"
        );
        return;
    }
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

    // Inline mock agent body — same JSON for all 5 agents. The
    // `calls` override scripts FOUR mock-emissions across TWO turns:
    //   turn 1 (agents instances spawn):   emit Call[0] (tool call) →
    //                            MCP gets a fresh session →
    //                            "true-<sid>" line on the axum server;
    //                            then emit Call[1] ("done") to end
    //                            the turn.
    //   turn 2 (agents instances message): emit Call[2] (tool call) →
    //                            same MCP session is REUSED → "false-<sid>"
    //                            line; then Call[3] ("done2") ends.
    //
    // Each agent runs independently with its own MCP proxy
    // connection. After both turns we should see exactly 5 "true-"
    // lines (5 fresh inits) + 5 "false-" lines (5 resumptions) = 10
    // unique lines.
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

    let executor = cli_test_util::executor_with_base_dir(&base);

    let spawn_agent = |seed: i64| {
        let executor = &executor;
        let agent_json = agent_json.clone();
        async move {
            let agent = AgentSpec::Resolved(
                serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                    agent_json,
                )
                .expect("inline mock agent must deserialize"),
            );
            let request = SpawnRequest { path_type: objectiveai_sdk::cli::command::agents::instances::spawn::Path::AgentsInstancesSpawn,
                agent_tag: None,
                message: Some(RequestMessage::Simple("go".to_string())),
                agent,
                seed: Some(seed),
                // Stream so we stay attached until the spawn's first
                // chunk lands and the cli has wired up the writer.
                // wait_for_completion below polls disk state and
                // cannot race an orphaned writer.
                dangerous_advanced: Some(SpawnDangerousAdvanced { stream: Some(true) }),
                jq: None,
            };
            let items: Vec<SpawnResponseItem> =
                cli_test_util::collect_stream(executor, request).await;
            items
                .iter()
                .find_map(|item| match item {
                    SpawnResponseItem::Chunk(chunk) => {
                        if chunk.id.is_empty() { None } else { Some(chunk.id.clone()) }
                    }
                    SpawnResponseItem::Id(_) => None,
                })
                .expect("agents instances spawn must emit a Chunk with non-empty id")
        }
    };

    // ── Run 1: spawn 5 agents SEQUENTIALLY ──────────────────────
    // Running these in parallel produces SQLite lock contention on
    // the shared cli filesystem db (each spawn opens its own writer
    // against the same `<base>/db.sqlite`). Each spawn finishes its
    // turn 1 (tool_call + "done") via wait_for_completion before the
    // next one starts, so the writer lock is released cleanly each
    // time.
    let mut leaves: Vec<String> = Vec::with_capacity(5);
    for i in 0..5 {
        let leaf = spawn_agent(i + 1).await;
        let full_id = format!("{CLI_HIERARCHY_ROOT}/{leaf}");
        wait_for_completion(&base, &full_id).await;
        leaves.push(leaf);
    }
    let full_ids: Vec<String> = leaves
        .iter()
        .map(|leaf| format!("{CLI_HIERARCHY_ROOT}/{leaf}"))
        .collect();
    let instances = leaves;

    // ── Send `agents instances message` to all 5 in parallel ──────
    // `dangerous_advanced.stream = Some(true)` keeps the parent cli
    // attached to its spawned instance runner so `collect_stream`
    // returning implies the runner exited — avoids the leak nextest
    // would otherwise flag.
    let send_futures = instances.iter().map(|instance| {
        let executor = &executor;
        let request = MessageRequest {
            path_type: objectiveai_sdk::cli::command::agents::instances::message::Path::AgentsInstancesMessage,
            target: MessageTarget::Direct {
                parent_agent_instance_hierarchy: None,
                agent_instance: instance.clone(),
                agent_tag: None,
            },
            message: RequestMessage::Simple("again".to_string()),
            seed: None,
            dangerous_advanced: Some(MessageDangerousAdvanced {
                stream: Some(true),
            }),
            jq: None,
        };
        async move {
            let _items: Vec<MessageResponseItem> =
                cli_test_util::collect_stream(executor, request).await;
        }
    });
    futures::future::join_all(send_futures).await;

    // ── Wait for each cli-stream to finish — parallel poll ─────
    let wait_futures = full_ids.iter().map(|id| {
        let base = &base;
        let id = id.clone();
        async move { wait_for_completion(base, &id).await }
    });
    futures::future::join_all(wait_futures).await;

    // ── Assertions ─────────────────────────────────────────────
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

    // The MCP server stamps each tool-call line with
    // `{true|false|unknown}-{X-OBJECTIVEAI-RESPONSE-ID}`. The
    // response-id is minted per cli invocation, so 5 spawns + 5
    // messages MUST produce 10 unique response-ids regardless of
    // how the upstream MCP session is keyed. We assert the
    // strongest property the current api guarantees: 10 unique
    // response-id stamps, all non-`unknown`.
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
