//! Five identical mock agents in one vector.completion task all dial
//! the SAME in-process axum MCP server. The test runs the function
//! execution once (5 agents → 5 tool calls in turn 1, "done" in
//! turn 2), then asks the cli `agents list active` for the five
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
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::agents::list::active::{
    Request as ListActiveRequest, ResponseItem as ListActiveItem,
};
use objectiveai_sdk::cli::command::agents::message::{
    Request as MessageRequest, RequestMessage, Response as MessageResponse,
};
use objectiveai_sdk::cli::command::functions::executions::create::standard::{
    Request, RequestInput, ResponseItem,
};
use objectiveai_sdk::cli::command::functions::executions::create::{
    FunctionSpec, ProfileSpec,
};
use objectiveai_sdk::functions::{
    FullInlineFunctionOrRemoteCommitOptional, InlineProfileOrRemoteCommitOptional,
};
use serde_json::{Value, json};
use tokio::sync::Mutex;

const SERVER_NAME: &str = "srv";
const TOOL_NAME: &str = "ping";
/// Default cli `agent_instance_hierarchy` root, set by
/// `ConfigBuilder::build` (`objectiveai-cli/src/run.rs:103-105`) when
/// `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY` is not set. The
/// `executor_with_base_dir` helper does NOT set that env var, so the
/// cli child uses this value as the parent for `agents list active`
/// and we have to re-prepend it to each returned `agent_id` before
/// passing it to `agents message` (which expects the FULL hierarchy).
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
async fn wait_for_completion(base: &Path, full_hierarchy: &str) {
    let cont = base
        .join("logs/agents/completions/response/continuation")
        .join(format!("{full_hierarchy}.txt"));
    let socket = base.join("pipes/cli").join(full_hierarchy).join("socket");
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if cont.exists() && !socket.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!(
        "cli-stream did not flush continuation + tear down socket for {full_hierarchy} in 180s"
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
    let _ = cli_test_util::cli_binary();

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
    // `calls` override scripts FOUR turns:
    //   turn 1 (function execution): tool call.
    //   turn 2 (function execution): "done" — ends fn-exec.
    //   turn 3 (agents message):     tool call.
    //   turn 4 (agents message):     "done2" — ends continuation.
    let prefixed_tool = format!("{SERVER_NAME}_{TOOL_NAME}");
    let agent = json!({
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
    let agents: Vec<Value> = (0..5).map(|_| agent.clone()).collect();
    let profile_json = json!({
        "agents": agents,
        "weights": [1, 1, 1, 1, 1]
    });
    let function_json = json!({
        "type": "vector.function",
        "tasks": [{
            "type": "vector.completion",
            "messages": [{ "role": "user", "content": "go" }],
            "responses": ["a", "b"],
            "output": { "$special": "Output" }
        }]
    });

    let executor = cli_test_util::executor_with_base_dir(&base);

    // ── Run 1: function execution ────────────────────────────────
    let function = FunctionSpec::Resolved(
        serde_json::from_value::<FullInlineFunctionOrRemoteCommitOptional>(function_json.clone())
            .expect("function JSON must deserialize"),
    );
    let profile = ProfileSpec::Resolved(
        serde_json::from_value::<InlineProfileOrRemoteCommitOptional>(profile_json.clone())
            .expect("profile JSON must deserialize"),
    );
    let request = Request { path: objectiveai_sdk::cli::command::functions::executions::create::standard::Path::FunctionsExecutionsCreateStandard,
        function,
        profile,
        input: RequestInput::Inline(
            serde_json::from_value(json!({})).expect("empty input deserializes"),
        ),
        continuation: None,
        retry_token: None,
        seed: Some(42),
        split: false,
        invert: false,
        dangerous_advanced: None,
        jq: None,
    };
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    assert!(
        !items.is_empty(),
        "function executor must emit at least one chunk"
    );

    // ── List active agents — must be exactly 5 ─────────────────
    let list_request = ListActiveRequest { path: objectiveai_sdk::cli::command::agents::list::active::Path::AgentsListActive,
        parent_agent_instance_hierarchy: None,
        jq: None,
    };
    let actives: Vec<ListActiveItem> =
        cli_test_util::collect_stream(&executor, list_request).await;
    assert_eq!(
        actives.len(),
        5,
        "expected exactly 5 active agents after function execution, got {}: {actives:?}",
        actives.len(),
    );

    // The handler strips the parent prefix from each `agent_id`, so
    // re-prepend the cli's hierarchy root to recover the full id
    // `agents message` expects.
    let full_ids: Vec<String> = actives
        .iter()
        .map(|a| format!("{CLI_HIERARCHY_ROOT}/{}", a.agent_id))
        .collect();

    // ── Send `agents message` to all 5 in parallel ─────────────
    let send_futures = full_ids.iter().map(|id| {
        let executor = &executor;
        let request = MessageRequest { path: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
            agent_instance_hierarchy: id.clone(),
            message: RequestMessage::Simple("again".to_string()),
            seed: None,
            jq: None,
        };
        async move {
            executor
                .execute_one::<_, MessageResponse>(request, None)
                .await
                .expect("agents message executor call")
        }
    });
    let _send_results: Vec<MessageResponse> = futures::future::join_all(send_futures).await;

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

    assert_eq!(
        unique.len(),
        10,
        "expected 10 unique lines across function execution + 5 continuation messages, \
         got {} unique from {} total lines (true={trues}, false={falses}, unknown={unknowns}): {lines:?}",
        unique.len(),
        lines.len(),
    );
    assert_eq!(
        trues, 5,
        "expected 5 lines starting `true-` (function execution fresh inits), got {trues} from {lines:?}",
    );
    assert_eq!(
        falses, 5,
        "expected 5 lines starting `false-` (per-agent message resumptions), got {falses} from {lines:?}",
    );
}
