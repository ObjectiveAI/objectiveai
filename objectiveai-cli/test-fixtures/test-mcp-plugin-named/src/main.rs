//! Test fixture plugin: serves a single-tool MCP endpoint whose
//! `serverInfo.name` echoes the `--name <value>` argv it was launched
//! with. Used by `agents_duplicate_tool_names_e2e` to stamp N
//! distinct upstream prefixes onto one common inner tool name
//! (`invoke`) so the test can drive proxy-side routing across
//! duplicate-named tools.
//!
//! ## CLI contract
//!
//! ```text
//! test-mcp-plugin-named mcp <mcp_name> begin --name <value>
//! ```
//!
//! `<value>` becomes the upstream's `serverInfo.name` (and the
//! `Mcp-Session-Id` for log greppability). The `<mcp_name>`
//! positional is ignored — only `--name` matters.
//!
//! Stdout (one JSON envelope, then the server runs):
//!
//! ```json
//! {"type":"mcp","url":"http://127.0.0.1:<port>"}
//! ```
//!
//! ## Server behavior
//!
//! - `initialize` → 200 + `Mcp-Session-Id: <name>` response header.
//! - `tools/list` → one tool named `invoke` with an open object
//!   schema.
//! - `tools/call` for `invoke` → 200 with `"ok"`. No side effects;
//!   the test asserts on continuation contents, not filesystem state.
//! - `notifications/initialized` → 202.
//! - DELETE → 200. Drop-time cleanup from the SDK lands here.
//! - everything else → 404.

use std::io::Write;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, post},
};
use serde_json::Value;

#[derive(Clone)]
struct AppState {
    name: Arc<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    // Parse `mcp <mcp_name> begin --name <value>` per the
    // `dial_plugin_upstream` contract in
    // `objectiveai-cli/src/instance/api/conduit.rs`. The first three
    // positional args are fixed; `--name` is the only flag we read.
    let mut args = std::env::args().skip(1);
    let _mcp = args.next();
    let _mcp_name = args.next();
    let _begin = args.next();
    let mut name: Option<String> = None;
    while let Some(arg) = args.next() {
        if arg == "--name" {
            name = args.next();
        }
    }
    let name = Arc::new(name.expect("fixture requires --name <value>"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    println!(r#"{{"type":"mcp","url":"http://{addr}"}}"#);
    std::io::stdout().flush()?;

    let app = Router::new()
        .route("/", post(handle_post))
        .route("/", delete(|| async { StatusCode::OK }))
        .with_state(AppState { name });
    axum::serve(listener, app).await
}

async fn handle_post(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
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
                    "serverInfo": { "name": *state.name, "version": "0.0.0" }
                }
            }))
            .into_response();
            // `Mcp-Session-Id` is forwarded by the cli conduit on
            // every later request to this upstream. Use the name so
            // the value is greppable in logs; the cli doesn't care
            // about the format.
            resp.headers_mut().insert(
                "Mcp-Session-Id",
                HeaderValue::from_str(&state.name).unwrap(),
            );
            resp
        }
        "notifications/initialized" => StatusCode::ACCEPTED.into_response(),
        "tools/list" => Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [{
                    "name": "invoke",
                    "description": "no-op test tool",
                    "inputSchema": { "type": "object", "additionalProperties": true }
                }]
            }
        }))
        .into_response(),
        "tools/call" => Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": "ok" }],
                "isError": false
            }
        }))
        .into_response(),
        other => (StatusCode::NOT_FOUND, format!("unknown method {other}")).into_response(),
    }
}
