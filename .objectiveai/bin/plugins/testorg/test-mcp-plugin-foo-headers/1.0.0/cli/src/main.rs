//! Test fixture plugin: serves an MCP-over-Streamable-HTTP endpoint
//! whose `Mcp-Session-Id` echoes the `--foo` argument the host passes,
//! and whose only tool (`invoke`) appends one line per call to
//! `<state_dir>/<foo>.txt`.
//!
//! ## CLI contract
//!
//! ```text
//! test-mcp-plugin-foo-headers mcp <name> begin --foo <value>
//! ```
//!
//! The `--foo <value>` argv is the materialized form of the
//! `X-OBJECTIVEAI-ARGUMENTS` map the API attaches to its initialize
//! POST — the CLI conduit reads that header off `init.args` and
//! translates each entry into `--<k> [v]` when it spawns this binary.
//!
//! Stdout (one JSON envelope, then the server runs):
//!
//! ```json
//! {"type":"mcp","url":"http://127.0.0.1:<port>"}
//! ```
//!
//! ## Server behavior
//!
//! - `initialize` → 200 + `Mcp-Session-Id: <foo>` response header.
//! - `tools/list` → one tool named `invoke` with an open object schema.
//! - `tools/call` for `invoke` → asserts incoming `Mcp-Session-Id`
//!   equals `<foo>` (returns 400 otherwise), then appends
//!   `"<foo> - <session-id>\n"` to `<state_dir>/<foo>.txt`.
//! - `notifications/initialized` → 202.
//! - everything else → 404.
//!
//! `<state_dir>` is the `STATE_DIR` env the host cli stamps on every
//! plugin spawn (`<dir>/state/<state>/plugins/<owner>/<name>/<version>`)
//! — the plugin's own install folder is committed and must never be
//! written to. Per-test-state isolation comes for free.

use std::io::Write;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::Value;

#[derive(Clone)]
struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    foo: String,
    base_dir: std::path::PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    // --- Parse `mcp <name> begin --foo <value>` ---
    //
    // The dial_plugin_upstream contract is
    //   <binary> mcp <mcp_name> begin --<key> <value>
    // per `objectiveai-cli/src/instance/api/conduit.rs`. We skip the
    // first three positional args (mcp, <name>, begin) and pull
    // `--foo` out of the rest.
    let mut args = std::env::args().skip(1);
    let _mcp = args.next();
    let _name = args.next();
    let _begin = args.next();
    let mut foo: Option<String> = None;
    while let Some(arg) = args.next() {
        if arg == "--foo" {
            foo = args.next();
        }
    }
    let foo = foo.expect("plugin requires --foo <value>");

    // --- STATE_DIR is stamped by the host cli on every spawn ---
    let base_dir = std::env::var("STATE_DIR")
        .map(std::path::PathBuf::from)
        .expect("plugin requires STATE_DIR env");

    let state = AppState {
        inner: Arc::new(Inner {
            foo: foo.clone(),
            base_dir,
        }),
    };

    // --- Bind on an ephemeral port; emit the plugin output ---
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let url = format!("http://{addr}");

    // Stdout protocol: one JSON envelope. The host's
    // `dial_plugin_upstream` reads the first non-empty stdout line
    // and deserializes as `cli::plugins::Output`. Field order is
    // hand-stamped here so the test doesn't depend on whatever
    // `serde_json::to_string` happens to emit. The SDK's
    // `cli::plugins::Mcp` shape is `{ url }` only — there is no
    // `headers` field anymore; per-request headers ride through the
    // proxy/conduit chain on their own.
    let line = format!(r#"{{"type":"mcp","url":"{url}"}}"#);
    println!("{line}");
    std::io::stdout().flush()?;

    let app = Router::new()
        .route("/", post(handle_post))
        // Generic OK on DELETE so the SDK's `Connection::delete` /
        // drop-time orphan-DELETE don't surface a 405 in test logs.
        .route("/", axum::routing::delete(handle_delete))
        .with_state(state);
    axum::serve(listener, app).await
}

async fn handle_delete() -> Response {
    StatusCode::OK.into_response()
}

async fn handle_post(
    State(state): State<AppState>,
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
                    "serverInfo": { "name": "test-foo-plugin", "version": "0.0.0" },
                }
            }))
            .into_response();
            // Session id = the `--foo` value, by design. The MCP
            // host (the proxy here) stamps this back on every later
            // request as `Mcp-Session-Id`.
            resp.headers_mut().insert(
                "Mcp-Session-Id",
                HeaderValue::from_str(&state.inner.foo).unwrap(),
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
                    "description": "Test fixture tool. Each call appends a line to <foo>.txt.",
                    "inputSchema": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }]
            }
        }))
        .into_response(),
        "tools/call" => {
            // Validate the routing invariant: the inbound
            // `Mcp-Session-Id` must equal the `--foo` value this
            // plugin instance was launched with. We mint the session
            // id off `foo` on initialize, the proxy stamps it on
            // every later request, so a mismatch here would mean a
            // call landed on the wrong plugin instance.
            let sid = headers
                .get("Mcp-Session-Id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            if sid != state.inner.foo {
                return (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "session id mismatch: Mcp-Session-Id={sid:?}, expected {:?}",
                        state.inner.foo,
                    ),
                )
                    .into_response();
            }

            // Append one line per call. Multiple calls in one test
            // run → multiple lines in the same file.
            let path = state.inner.base_dir.join(format!("{}.txt", state.inner.foo));
            let line = format!("{} - {}\n", state.inner.foo, sid);
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .expect("open output file");
            f.write_all(line.as_bytes()).expect("write");

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
        other => (
            StatusCode::NOT_FOUND,
            format!("unknown method {other}"),
        )
            .into_response(),
    }
}
