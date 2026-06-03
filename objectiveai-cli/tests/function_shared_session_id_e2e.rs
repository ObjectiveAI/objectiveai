//! Five identical mock agents in one vector.completion task all dial
//! the SAME in-process axum MCP server. The test runs the function
//! execution twice — once fresh, once with a continuation token
//! sourced from the prior run's per-agent response-continuation file.
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
//!      across two runs of the same swarm (10 unique response ids).
//!   2. The proxy sends the prior `Mcp-Session-Id` header on the
//!      continuation run (5 `false-` lines).
//!   3. The proxy starts fresh without a `Mcp-Session-Id` header
//!      when no prior session is supplied (5 `true-` lines).

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

/// Server state. `is_new_by_session` keyed by the server-minted
/// `Mcp-Session-Id` returned on initialize. Set once at init time;
/// read at every `tools/call` to label that call's file-line.
#[derive(Clone)]
struct ServerState {
    output_path: Arc<PathBuf>,
    is_new_by_session: Arc<Mutex<HashMap<String, bool>>>,
}

fn temp_base() -> PathBuf {
    let d = std::env::temp_dir().join(format!("oai-shared-sid-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// RAII cleanup for the per-test scratch dir. Mirrors
/// `plugin_mcp_dispatch_e2e::PluginGuard`'s scratch-dir half (no PID
/// file — axum runs in our own tokio runtime and dies with it).
struct BaseGuard(PathBuf);

impl Drop for BaseGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
            // header → resumption (proxy is telling us to resume a
            // prior session); absent → fresh client. Mint a fresh
            // server-side session id on EVERY init so run 1 and
            // run 2 ids are guaranteed distinct.
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
            // Look up is_new for this session. Missing entry would
            // mean a tools/call arrived with an unknown session id —
            // record the `unknown-` bucket so the assertion catches
            // it loudly instead of silently passing.
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

/// Walk the per-agent response-continuation directory and return the
/// raw bytes of the first non-empty `.txt` file we find. The cli
/// persists each agent's response continuation as raw UTF-8 at
/// `<base>/logs/agents/completions/response/continuation/<id>.txt`
/// (see `objectiveai-cli/src/filesystem/logs/latest_continuation.rs`
/// for the canonical path). For a vector.completion with five
/// identical agents driven through the same `calls` override, the
/// per-agent continuation envelopes are interchangeable for resuming
/// the function-level conversation.
async fn read_any_continuation(base: &Path) -> Option<String> {
    let dir = base.join("logs/agents/completions/response/continuation");
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                    if let Ok(s) = std::fs::read_to_string(&path) {
                        if !s.is_empty() {
                            return Some(s);
                        }
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    None
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

    let base = temp_base();
    let _cleanup = BaseGuard(base.clone());

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

    // Inline mock agent body — same JSON for all 5 agents. Tool name
    // is `<serverInfo.name>_<tool>` per the proxy's prefix rule.
    let prefixed_tool = format!("{SERVER_NAME}_{TOOL_NAME}");
    let agent = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "mcp_servers": [
            { "url": url, "authorization": false }
        ],
        "calls": [
            {
                "tool_calls": [
                    { "name": prefixed_tool, "arguments": "{}" }
                ],
                "content": ""
            },
            {
                "tool_calls": [],
                "content": "done"
            }
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

    let build_request = |continuation: Option<String>| -> Request {
        let function = FunctionSpec::Resolved(
            serde_json::from_value::<FullInlineFunctionOrRemoteCommitOptional>(
                function_json.clone(),
            )
            .expect("function JSON must deserialize"),
        );
        let profile = ProfileSpec::Resolved(
            serde_json::from_value::<InlineProfileOrRemoteCommitOptional>(
                profile_json.clone(),
            )
            .expect("profile JSON must deserialize"),
        );
        Request {
            function,
            profile,
            input: RequestInput::Inline(
                serde_json::from_value(json!({})).expect("empty input deserializes"),
            ),
            continuation,
            retry_token: None,
            seed: Some(42),
            split: false,
            invert: false,
            dangerous_advanced: None,
            jq: None,
        }
    };

    // Run 1 — fresh; no continuation. The proxy dials upstream with
    // `connect(url, None, headers)`; SDK sends initialize without an
    // `Mcp-Session-Id` header → server marks each minted id as
    // `is_new=true` → file lines `"true-<response_id>"`.
    let items1: Vec<ResponseItem> =
        cli_test_util::collect_stream(&executor, build_request(None)).await;
    assert!(
        !items1.is_empty(),
        "run 1 must emit at least one chunk"
    );

    // Source a continuation token from any per-agent response file.
    // All five share the same shape since the agents run the same
    // `calls` script — picking any one resumes the function-level
    // conversation.
    let token = read_any_continuation(&base)
        .await
        .expect("run 1 must produce at least one per-agent continuation file");

    // Run 2 — continuation. The API decodes the token, threads each
    // agent's prior session ids forward, and the SDK now calls
    // `connect(url, Some(prior_session_id), headers)`. SDK sends the
    // prior id as the `Mcp-Session-Id` header on its initialize POST
    // → server marks each minted id as `is_new=false` → file lines
    // `"false-<response_id>"`.
    let items2: Vec<ResponseItem> =
        cli_test_util::collect_stream(&executor, build_request(Some(token))).await;
    assert!(
        !items2.is_empty(),
        "run 2 must emit at least one chunk"
    );

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
        "expected 10 unique lines across two runs of 5 agents each, \
         got {} unique from {} total lines (true={trues}, false={falses}, unknown={unknowns}): {lines:?}",
        unique.len(),
        lines.len(),
    );
    assert_eq!(
        trues, 5,
        "expected 5 lines starting `true-` (run 1 fresh), got {trues} from {lines:?}",
    );
    assert_eq!(
        falses, 5,
        "expected 5 lines starting `false-` (run 2 resumption), got {falses} from {lines:?}",
    );
}
