//! Test fixture plugin: serves an MCP-over-Streamable-HTTP endpoint
//! whose `Mcp-Session-Id` echoes the `--foo` argument the host passes,
//! and whose only tool (`invoke`) appends one line per call to
//! `<CONFIG_BASE_DIR>/<foo>.txt`.
//!
//! ## CLI contract
//!
//! ```text
//! test-mcp-plugin-foo-headers mcp <name> begin --foo <value>
//! ```
//!
//! Stdout (one JSON envelope, then the server runs):
//!
//! ```json
//! {"type":"mcp","url":"http://127.0.0.1:<port>","headers":{"X-FOO":"<value>"}}
//! ```
//!
//! ## Server behavior
//!
//! - `initialize` → 200 + `Mcp-Session-Id: <foo>` response header.
//! - `tools/list` → one tool named `invoke` with an open object schema.
//! - `tools/call` for `invoke` → asserts incoming `X-FOO` and
//!   `Mcp-Session-Id` both equal `<foo>` (returns 400 otherwise), then
//!   appends `"<foo> - <session-id>\n"` to `<CONFIG_BASE_DIR>/<foo>.txt`.
//! - `notifications/initialized` → 202.
//! - everything else → 404.
//!
//! `<CONFIG_BASE_DIR>` is read from the `CONFIG_BASE_DIR` env var, which
//! the host cli inherits to its `dial_plugin_upstream` child and we
//! inherit transitively from that child.

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

    // --- CONFIG_BASE_DIR is inherited from the cli's env ---
    let base_dir = std::env::var("CONFIG_BASE_DIR")
        .map(std::path::PathBuf::from)
        .expect("plugin requires CONFIG_BASE_DIR env");

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
    // `serde_json::to_string` happens to emit.
    let line = format!(
        r#"{{"type":"mcp","url":"{url}","headers":{{"X-FOO":"{foo}"}}}}"#,
    );
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
            // Validate the routing invariants: both headers must
            // equal the `--foo` value this plugin instance was
            // launched with. The whole point of the fixture is to
            // prove the host-side plugin → MCP → proxy header
            // plumbing is wiring these correctly.
            let sid = headers
                .get("Mcp-Session-Id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            let xfoo = headers
                .get("X-FOO")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            if sid != state.inner.foo || xfoo != state.inner.foo {
                return (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "header mismatch: X-FOO={xfoo:?}, Mcp-Session-Id={sid:?}, expected {:?}",
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
