//! The resident daemon's broadcast hub.
//!
//! Two listeners share one [`broadcast`] channel of already-serialized
//! JSON frames:
//!
//! - **Producer side** — a fixed-name local socket (`<state>/socks/daemon.sock`
//!   on Unix; a namespaced pipe on Windows). A producer connects, streams
//!   its agent/plugin **context** object, then one CLI **request** line,
//!   then that request's CLI **response** lines (newline-delimited JSON,
//!   no ack), then closes. `interprocess` inserts no framing of its own,
//!   so the trailing `\n` is the only delimiter — the same wire shape as
//!   [`crate::websockets::mcp_listener`].
//! - **Consumer side** — an [`axum`] WebSocket server bound to the
//!   daemon's configured `address:port`, single root endpoint (`/`).
//!   Every client that connects immediately begins receiving future
//!   frames; it is a pure push channel (inbound messages are ignored
//!   except to notice the client closing).
//!
//! Each producer connection is assigned a fresh `id`. The request is
//! wrapped as the SDK [`RootViewerRequest`] (`{…context, id, value}` —
//! the producer's context fields stamped alongside `id`); every following
//! item as [`RootViewerResponseItem`] (`{id, path_type, value}`), where
//! `path_type` is lifted off that connection's opening request. The `id`
//! lets a consumer demultiplex concurrent producer streams; `path_type`
//! tags each response with the command that produced it.
//!
//! Frames are validated through the SDK Viewer types when the command is
//! recognized and passed through raw otherwise (forward-compat). The
//! underlying items stay opaque [`serde_json::Value`]s on the wire, so it
//! stays
//! forward-compatible with command shapes it predates.

use std::path::Path;

#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{ListenerOptions, Name};
use objectiveai_sdk::cli::command::{RootViewerRequest, RootViewerResponseItem};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;

use crate::websockets::mcp_listener::socks_dir;

/// The fixed local-socket name for the daemon hub, identical on the
/// listener and producer sides. Unix uses a filesystem socket under
/// `<state>/socks/daemon.sock`; Windows local sockets are named pipes
/// (no filesystem home), so it uses a namespaced pipe name keyed by the
/// state name — mirroring [`crate::websockets::mcp_listener`], but with
/// the constant `daemon` in place of a `response_id`.
#[cfg(unix)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    socks_dir(state_dir)
        .join("daemon.sock")
        .to_fs_name::<GenericFilePath>()
}

#[cfg(windows)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    use std::hash::{Hash, Hasher};
    // Named pipes are machine-global, so fold the state NAME into the
    // pipe name to preserve the per-state isolation the Unix
    // `<state>/socks/` path gives. Hash the final path component only, so
    // the same state maps to the same pipe across machines (and matches
    // `mcp_listener`'s scheme).
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state_dir.file_name().hash(&mut hasher);
    let state = hasher.finish();
    format!("objectiveai-{state:016x}-daemon.sock").to_ns_name::<GenericNamespaced>()
}

/// Bind the fixed-name producer socket, returning the bound listener.
/// Binding is **synchronous** so the daemon can publish its lock only
/// AFTER the socket is listening: a held daemon lock then guarantees the
/// socket is up, so a producer either connects on the first try or the
/// daemon is dead — no connect retry needed. `try_overwrite` clears a
/// stale socket file left by a crashed predecessor (the singleton daemon
/// lock guarantees no live peer).
pub fn bind_socket_listener(
    state_dir: &Path,
) -> std::io::Result<interprocess::local_socket::tokio::Listener> {
    // Ensure the socks dir exists for the Unix filesystem socket; harmless
    // on Windows (which uses a namespaced pipe name).
    let _ = std::fs::create_dir_all(socks_dir(state_dir));
    let name = socket_name(state_dir)?;
    ListenerOptions::new()
        .name(name)
        .try_overwrite(true)
        .create_tokio()
}

/// Spawn the accept loop on a pre-bound producer socket: one
/// [`handle_feed`] task per connection, fanning wrapped items onto `tx`.
pub fn serve_socket_listener(
    listener: interprocess::local_socket::tokio::Listener,
    tx: broadcast::Sender<String>,
) {
    tokio::spawn(async move {
        loop {
            let conn = match listener.accept().await {
                Ok(conn) => conn,
                // Transient accept error — keep serving.
                Err(_) => continue,
            };
            tokio::spawn(handle_feed(conn, tx.clone()));
        }
    });
}

/// Serve one producer connection. The producer's FIRST line is its
/// agent/plugin **context** object (the fields the request wrapper
/// carries — `agent_instance_hierarchy`, `response_id`, `plugin_*`, …);
/// the SECOND line is the CLI request; the rest are CLI responses. The
/// request is broadcast as a [`RootViewerRequest`] (`{…context, id,
/// value}`), each response as a [`RootViewerResponseItem`] (`{id,
/// path_type, value}`). No writes back — the producer streams and closes
/// with no ack. EOF ends the task.
async fn handle_feed(conn: LocalSocketStream, tx: broadcast::Sender<String>) {
    let (read_half, _write_half) = tokio::io::split(conn);
    let mut reader = BufReader::new(read_half);
    let id = uuid::Uuid::new_v4().to_string();
    let mut line = String::new();

    // First line: the producer's context object. Absent / malformed /
    // non-object → empty context (the wrapper's context fields are all
    // optional). Fields the wrapper doesn't declare (e.g. mcp_session_id)
    // are dropped when the envelope deserializes.
    let context: serde_json::Map<String, serde_json::Value> = loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => return, // producer closed before sending anything
            Ok(_) => {}
            Err(_) => return,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        break match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(serde_json::Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        };
    };

    // `None` until the opening request is read; then holds its `path`.
    let mut path: Option<String> = None;
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF: producer closed.
            Ok(_) => {}
            Err(_) => break,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            // Skip a malformed line rather than tearing down the stream.
            Err(_) => continue,
        };
        let frame = match &path {
            None => {
                // First item after the context = the CLI request. Lift
                // its `path_type` (the shared command-path string) to tag
                // every response.
                let p = value
                    .get("path_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                path = Some(p);
                // Wrap as a `RootViewerRequest` (`{…context, id, value}`).
                // Validate through the SDK type when the command is known;
                // fall back to the raw envelope for forward-compat with
                // commands this binary predates.
                let mut envelope = context.clone();
                envelope.insert("id".to_string(), serde_json::json!(id.clone()));
                envelope.insert("value".to_string(), value);
                let envelope = serde_json::Value::Object(envelope);
                match serde_json::from_value::<RootViewerRequest>(envelope.clone()) {
                    Ok(vr) => serde_json::to_string(&vr),
                    Err(_) => Ok(envelope.to_string()),
                }
            }
            Some(p) => {
                // Wrap as a `RootViewerResponseItem` ({id, path_type,
                // value}), same validate-or-passthrough treatment.
                let envelope = serde_json::json!({
                    "id": id.clone(),
                    "path_type": p.clone(),
                    "value": value,
                });
                match serde_json::from_value::<RootViewerResponseItem>(envelope.clone()) {
                    Ok(vri) => serde_json::to_string(&vri),
                    Err(_) => Ok(envelope.to_string()),
                }
            }
        };
        if let Ok(frame) = frame {
            // A send error means no WebSocket clients are connected —
            // nothing to fan out to. Drop the frame.
            let _ = tx.send(frame);
        }
    }
}

/// Serve the consumer side: an axum WebSocket server on `listener`,
/// single root endpoint. Returns the serve task's handle. Each accepted
/// client subscribes to `tx` and receives every future broadcast frame.
/// When `secret` is `Some`, upgrades are gated by
/// [`crate::websockets::daemon_auth`]; when `None`, the server is open.
pub fn serve_ws(
    listener: tokio::net::TcpListener,
    tx: broadcast::Sender<String>,
    secret: Option<std::sync::Arc<String>>,
) -> tokio::task::JoinHandle<()> {
    let mut app = axum::Router::new()
        .route("/", axum::routing::any(ws_handler))
        .with_state(tx);
    // Optional auth: when a secret is configured, gate every upgrade on a
    // valid `sha256=<hex(SHA256(secret))>` signature header; otherwise the
    // server is open (no middleware layered).
    if let Some(secret) = secret {
        app = app.layer(axum::middleware::from_fn_with_state(
            Some(secret),
            crate::websockets::daemon_auth::signature_middleware,
        ));
    }
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    })
}

/// Root endpoint: upgrade to WebSocket and pump broadcast frames.
async fn ws_handler(
    axum::extract::State(tx): axum::extract::State<broadcast::Sender<String>>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| pump(socket, tx))
}

/// Forward every broadcast frame to one client until it disconnects.
/// Pure push: inbound frames are read only to notice the close. A
/// `Lagged` broadcast receiver (slow client) drops missed frames and
/// keeps going.
async fn pump(mut socket: axum::extract::ws::WebSocket, tx: broadcast::Sender<String>) {
    use axum::extract::ws::Message;
    let mut rx = tx.subscribe();
    loop {
        tokio::select! {
            received = rx.recv() => match received {
                Ok(frame) => {
                    if socket.send(Message::Text(frame.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            inbound = socket.recv() => match inbound {
                // Client closed or errored.
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                // Ignore any other inbound message.
                Some(Ok(_)) => {}
            },
        }
    }
}

/// Producer/test helper: connect to the daemon socket, stream the
/// `context` object, then `request`, then each of `responses` as
/// newline-delimited JSON, and close. The inverse of [`handle_feed`].
/// `context` carries the producer's agent/plugin fields
/// (`agent_instance_hierarchy`, `response_id`, `plugin_*`, …) that the
/// request wrapper is stamped with.
pub async fn feed_socket(
    state_dir: &Path,
    context: &serde_json::Value,
    request: &serde_json::Value,
    responses: &[serde_json::Value],
) -> std::io::Result<()> {
    let name = socket_name(state_dir)?;
    let conn = LocalSocketStream::connect(name).await?;
    let (_read_half, mut write_half) = tokio::io::split(conn);
    write_line(&mut write_half, context).await?;
    write_line(&mut write_half, request).await?;
    for response in responses {
        write_line(&mut write_half, response).await?;
    }
    write_half.flush().await?;
    write_half.shutdown().await?;
    Ok(())
}

async fn write_line<W: AsyncWriteExt + Unpin>(
    write_half: &mut W,
    value: &serde_json::Value,
) -> std::io::Result<()> {
    let line = serde_json::to_string(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    write_half.write_all(line.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    Ok(())
}

/// Incremental producer handle: holds the daemon socket's write half so a
/// producer can stream items one at a time as they arrive (unlike
/// [`feed_socket`], which sends everything at once). Dropping it closes
/// the write half, so the daemon reads EOF and finalizes the stream.
pub struct FeedWriter {
    write: tokio::io::WriteHalf<LocalSocketStream>,
}

impl FeedWriter {
    /// Write one JSON value as a newline-delimited, flushed line. The
    /// line and its `\n` go out in a single buffer (one write per item —
    /// this sits behind every teed stream item).
    pub async fn write(&mut self, value: &serde_json::Value) -> std::io::Result<()> {
        let mut line = serde_json::to_vec(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push(b'\n');
        self.write.write_all(&line).await?;
        self.write.flush().await
    }
}

/// Connect to the daemon socket for incremental feeding.
///
/// No daemon-liveness retry: the daemon publishes its lock only after
/// this socket is listening (see [`bind_socket_listener`]), so if the
/// caller reached here after `daemon spawn` returned, the socket is up —
/// a refused/absent socket means the daemon is dead, and that fails
/// immediately.
///
/// The ONE retried error is Windows `ERROR_PIPE_BUSY` (231): named-pipe
/// listeners expose a finite number of instances and re-post one right
/// after each accept, so under concurrent producers a connect can land in
/// the instant every instance is taken. That state means the daemon is
/// ALIVE (a live listener is mid-accept) — the opposite of dead — so a
/// brief bounded retry is correct there and only there. Unix never
/// produces this code (connects queue in the listen backlog).
pub async fn connect_feed(state_dir: &Path) -> std::io::Result<FeedWriter> {
    const ERROR_PIPE_BUSY: i32 = 231;
    let mut attempts = 0u32;
    loop {
        let name = socket_name(state_dir)?;
        match LocalSocketStream::connect(name).await {
            Ok(conn) => {
                let (_read_half, write) = tokio::io::split(conn);
                return Ok(FeedWriter { write });
            }
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && attempts < 20 => {
                attempts += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
