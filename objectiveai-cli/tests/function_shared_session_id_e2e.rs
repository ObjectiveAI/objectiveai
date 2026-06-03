//! Five identical mock agents in one vector.completion task all dial
//! the SAME in-process axum MCP server, which returns a FIXED
//! `Mcp-Session-Id` on every initialize. Each agent's `calls`
//! override fires one tool call in turn 1 and emits "done" in turn 2.
//! The axum server appends every inbound `tools/call`'s
//! `X-OBJECTIVEAI-RESPONSE-ID` header to a file under
//! `CONFIG_BASE_DIR`. After the function executor returns, the test
//! reads the file and asserts exactly 5 unique response ids.
//!
//! Load-bearing invariant: per-agent identity (`X-OBJECTIVEAI-RESPONSE-ID`)
//! must survive the proxy's session-storage path even when every
//! agent's payload encrypts to the same proxy session id and all
//! five share one `Arc<Session>`. Failure surface = transient-bag
//! refresh racing between concurrent siblings, which would let two
//! tool calls land on the upstream carrying the same response id.

mod cli_test_util;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

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

const FIXED_SESSION_ID: &str = "fixed-shared-session-id";
const SERVER_NAME: &str = "srv";
const TOOL_NAME: &str = "ping";

#[derive(Clone)]
struct ServerState {
    output_path: Arc<PathBuf>,
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
            // Deliberate: every initialize across all five sibling
            // agents gets the SAME session id. That's the collision
            // we want to stress.
            resp.headers_mut().insert(
                "Mcp-Session-Id",
                HeaderValue::from_static(FIXED_SESSION_ID),
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
            let rid = headers
                .get("X-OBJECTIVEAI-RESPONSE-ID")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            // The output path was set from the test's base dir on
            // server startup; nothing outside CONFIG_BASE_DIR is
            // touched.
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(state.output_path.as_ref())
                .expect("open response-ids file");
            writeln!(f, "{rid}").expect("write response id");
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
async fn shared_mcp_session_preserves_per_agent_response_ids() {
    if cli_test_util::test_api_address().is_none() {
        eprintln!(
            "skipping shared_mcp_session_preserves_per_agent_response_ids: \
             OBJECTIVEAI_TEST_PORT not set"
        );
        return;
    }
    let _ = cli_test_util::cli_binary();

    let base = temp_base();
    let _cleanup = BaseGuard(base.clone());

    let output_path = Arc::new(base.join("response-ids.txt"));

    // Bind an ephemeral port and spawn axum on a background task.
    // The server task lives for the lifetime of the test's tokio
    // runtime; nothing has to kill it explicitly.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind axum");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let state = ServerState {
        output_path: output_path.clone(),
    };
    let app = Router::new()
        .route("/", post(handle_post))
        .route("/", delete(|| async { StatusCode::OK }))
        .with_state(state);
    let _server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Inline mock agent. `mcp_servers` is the direct-URL path (NOT
    // `client_objectiveai_mcp.plugins`), shape per
    // `objectiveai-sdk-rs/src/agent/mcp.rs`: `{ url, authorization }`.
    // The `calls` override fires one tool call in turn 1, "done"
    // content in turn 2 — same body across all five agents.
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

    // Five IDENTICAL agents. `count` defaults to 1 on
    // `InlineAgentBaseWithFallbacksOrRemoteWithCount`.
    let agents: Vec<Value> = (0..5).map(|_| agent.clone()).collect();
    let profile_json = json!({
        "agents": agents,
        "weights": [1, 1, 1, 1, 1]
    });

    // One-task vector function. `output` is a pass-through Special;
    // this test does not assert on the score vector.
    let function_json = json!({
        "type": "vector.function",
        "tasks": [{
            "type": "vector.completion",
            "messages": [{ "role": "user", "content": "go" }],
            "responses": ["a", "b"],
            "output": { "$special": "Output" }
        }]
    });

    let function = FunctionSpec::Resolved(
        serde_json::from_value::<FullInlineFunctionOrRemoteCommitOptional>(function_json)
            .expect("function JSON must deserialize"),
    );
    let profile = ProfileSpec::Resolved(
        serde_json::from_value::<InlineProfileOrRemoteCommitOptional>(profile_json)
            .expect("profile JSON must deserialize"),
    );
    let request = Request {
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

    let executor = cli_test_util::executor_with_base_dir(&base);
    let items: Vec<ResponseItem> = cli_test_util::collect_stream(&executor, request).await;
    assert!(
        !items.is_empty(),
        "function executor must emit at least one chunk"
    );

    let raw = std::fs::read_to_string(output_path.as_ref()).unwrap_or_default();
    let total = raw.lines().count();
    let unique: HashSet<String> = raw
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(
        unique.len(),
        5,
        "expected 5 unique X-OBJECTIVEAI-RESPONSE-ID values across 5 agent tool calls, \
         got {} unique from {} total lines: {unique:?}",
        unique.len(),
        total,
    );
}
