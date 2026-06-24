//! Per-`response_id` MCP listener: a local socket that forwards MCP
//! ops to the API over the streaming WebSocket.
//!
//! `agents spawn` and `functions execute` each open one WS to the API
//! and obtain a [`Notifier`]. As soon as a chunk surfaces a new
//! agent-completion `response_id`, the entry point calls
//! [`spawn_mcp_listener`], which binds a local socket at
//! `<state>/socks/<response_id>.sock` and forwards every request it
//! receives to the proxy (tagged with that `response_id`) over the
//! shared WS. The proxy's reply is written straight back.
//!
//! Protocol: exactly one request -> one response per connection. The
//! request is a single line of JSON ([`SocketRequest`], internally
//! tagged by the MCP method `path`); the response is a single line of
//! JSON ([`SocketResponse`]). `interprocess` inserts no framing of its
//! own, so the trailing `\n` is the only delimiter.
//!
//! Listeners are detached and live for the rest of the process — the
//! socket is the agent's MCP endpoint for as long as the spawning
//! command runs.
//!
//! Note: sometimes the caller will be within this same process. We
//! could skip the unix socket entirely in that case (call the
//! `Notifier` directly) if performance becomes important, but for now
//! routing everything through the socket keeps the logic shared and
//! simpler.

use std::path::{Path, PathBuf};

use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{ListenerOptions, Name};
#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use objectiveai_sdk::Notifier;
use objectiveai_sdk::client_objectiveai_mcp::server_response::JsonRpcResult;
use objectiveai_sdk::mcp;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// JSON-RPC server-error code reported on the socket for failures the
/// proxy never produced — transport teardown or a malformed request.
const SOCKET_ERR_CODE: i64 = -32099;

/// A request arriving on a per-`response_id` MCP socket. Internally
/// tagged by the MCP method `path` (`tools/list`, `tools/call`,
/// `resources/list`, `resources/read`); the op's params are flattened
/// alongside (newtype-of-struct flatten). The `response_id` is NOT on
/// the wire — the listener supplies it from the socket's filename.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "path")]
pub enum SocketRequest {
    #[serde(rename = "tools/list")]
    ListTools(mcp::tool::ListToolsRequest),
    #[serde(rename = "tools/call")]
    CallTool(mcp::tool::CallToolRequestParams),
    #[serde(rename = "resources/list")]
    ListResources(mcp::resource::ListResourcesRequest),
    #[serde(rename = "resources/read")]
    ReadResource(mcp::resource::ReadResourceRequestParams),
}

/// The reply written back on the socket: `{type, value}`. On success
/// `value` is the embedded MCP result as-is (one of the four result
/// types); on failure `value` is `{code, message}`. No transport
/// envelope, no JSON-RPC `data`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SocketResponse<R> {
    Ok(R),
    Err(McpError),
}

/// MCP error object embedded in [`SocketResponse::Err`].
#[derive(Debug, Serialize, Deserialize)]
pub struct McpError {
    pub code: i64,
    pub message: String,
}

/// `<state>/socks` — the per-state directory holding one
/// `<response_id>.sock` per live agent-completion listener. Mirrors
/// the `<state>/locks` layout in [`crate::command::agents::locks`].
pub fn socks_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("socks")
}

/// The local-socket name for a `response_id`, identical on the listener
/// and client sides. Unix uses a filesystem socket under
/// `<state>/socks/<response_id>.sock`; Windows local sockets are named
/// pipes (no filesystem home), so it uses a namespaced pipe name keyed
/// by the response id.
#[cfg(unix)]
fn socket_name(
    state_dir: &Path,
    response_id: &str,
) -> std::io::Result<Name<'static>> {
    socks_dir(state_dir)
        .join(format!("{response_id}.sock"))
        .to_fs_name::<GenericFilePath>()
}

#[cfg(windows)]
fn socket_name(
    state_dir: &Path,
    response_id: &str,
) -> std::io::Result<Name<'static>> {
    use std::hash::{Hash, Hasher};
    // Named pipes are machine-global (no per-state filesystem dir), so
    // fold the per-state dir into the name to preserve the same
    // isolation the Unix `<state>/socks/` path gives. `DefaultHasher` is
    // deterministic, so the listener and client derive the same name.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state_dir.hash(&mut hasher);
    let state = hasher.finish();
    format!("objectiveai-{state:016x}-{response_id}.sock")
        .to_ns_name::<GenericNamespaced>()
}

/// Client side of the protocol: connect to `<state>/socks/<response_id>.sock`,
/// send one [`SocketRequest`] line, read one [`SocketResponse`] line back.
/// The inverse of [`handle_conn`]; same `\n`-delimited, one-shot protocol.
///
/// A connect failure (the spawning process isn't running, or no listener
/// for this `response_id`) or a malformed reply surfaces as `io::Error`.
pub async fn call_socket<R: serde::de::DeserializeOwned>(
    state_dir: &Path,
    response_id: &str,
    request: &SocketRequest,
) -> std::io::Result<SocketResponse<R>> {
    let name = socket_name(state_dir, response_id)?;
    let conn = LocalSocketStream::connect(name).await?;
    let (read_half, mut write_half) = tokio::io::split(conn);

    let line = serde_json::to_string(request)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    write_half.write_all(line.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    write_half.flush().await?;

    let mut reader = BufReader::new(read_half);
    let mut reply = String::new();
    reader.read_line(&mut reply).await?;
    serde_json::from_str::<SocketResponse<R>>(reply.trim())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Bind `<state>/socks/<response_id>.sock` and forward every request
/// to the proxy over `notifier`'s WS, tagged with `response_id`. The
/// listener is detached and lives for the rest of the process.
///
/// Best-effort: any failure to create the directory or bind the
/// socket simply means no listener for this `response_id` — the
/// owning command stream is unaffected.
pub fn spawn_mcp_listener(
    response_id: String,
    notifier: Notifier,
    state_dir: PathBuf,
) {
    tokio::spawn(async move {
        // Ensure the socks dir exists for the Unix filesystem socket;
        // harmless on Windows (which uses a namespaced pipe name).
        let _ = tokio::fs::create_dir_all(socks_dir(&state_dir)).await;
        let Ok(name) = socket_name(&state_dir, &response_id) else {
            return;
        };
        // `reclaim_name` (on by default) removes the socket on drop;
        // `try_overwrite` clears a stale file left by a crashed
        // predecessor. Response ids are unique, so neither ever
        // displaces a live peer.
        let listener = match ListenerOptions::new()
            .name(name)
            .try_overwrite(true)
            .create_tokio()
        {
            Ok(l) => l,
            Err(_) => return,
        };
        loop {
            let conn = match listener.accept().await {
                Ok(conn) => conn,
                // Transient accept error — keep serving.
                Err(_) => continue,
            };
            tokio::spawn(handle_conn(
                conn,
                notifier.clone(),
                response_id.clone(),
            ));
        }
    });
}

/// Serve a single connection: read one request line, forward the op
/// to the proxy, write the reply line, close.
async fn handle_conn(
    conn: LocalSocketStream,
    notifier: Notifier,
    response_id: String,
) {
    let (read_half, mut write_half) = tokio::io::split(conn);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return;
    }

    let reply = match serde_json::from_str::<SocketRequest>(line.trim()) {
        Ok(SocketRequest::ListTools(params)) => {
            render(notifier.list_tools(response_id, params).await)
        }
        Ok(SocketRequest::CallTool(params)) => {
            render(notifier.call_tool(response_id, params).await)
        }
        Ok(SocketRequest::ListResources(params)) => {
            render(notifier.list_resources(response_id, params).await)
        }
        Ok(SocketRequest::ReadResource(params)) => {
            render(notifier.read_resource(response_id, params).await)
        }
        Err(e) => err_line(format!("malformed request: {e}")),
    };

    let _ = write_half.write_all(reply.as_bytes()).await;
    let _ = write_half.write_all(b"\n").await;
    let _ = write_half.shutdown().await;
}

/// Map a notifier MCP-op outcome into the on-socket `{type, value}`
/// reply text. The proxy's `JsonRpcResult::Err` and a transport
/// failure both render as `type:"err"`; only the code source differs
/// (the proxy's own code vs. [`SOCKET_ERR_CODE`]).
fn render<R, E>(result: Result<JsonRpcResult<R>, E>) -> String
where
    R: Serialize,
    E: std::fmt::Display,
{
    let response = match result {
        Ok(JsonRpcResult::Ok { result }) => SocketResponse::Ok(result),
        Ok(JsonRpcResult::Err { code, message, .. }) => {
            SocketResponse::Err(McpError { code, message })
        }
        Err(e) => SocketResponse::Err(McpError {
            code: SOCKET_ERR_CODE,
            message: e.to_string(),
        }),
    };
    serde_json::to_string(&response)
        .unwrap_or_else(|_| err_body(SOCKET_ERR_CODE, "serialize error"))
}

/// Build an err reply line for failures with no `R` (e.g. a malformed
/// request, before any op type is known).
fn err_line(message: String) -> String {
    serde_json::to_string(&SocketResponse::<()>::Err(McpError {
        code: SOCKET_ERR_CODE,
        message,
    }))
    .unwrap_or_else(|_| err_body(SOCKET_ERR_CODE, "serialize error"))
}

/// Last-resort hand-built err body for the (near-impossible) case that
/// serializing a [`SocketResponse`] itself fails.
fn err_body(code: i64, message: &str) -> String {
    format!(
        r#"{{"type":"err","value":{{"code":{code},"message":"{message}"}}}}"#
    )
}
