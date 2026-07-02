//! The resident daemon's broadcast hub.
//!
//! Two listeners share one [`broadcast`] channel of already-serialized
//! JSON frames:
//!
//! - **Producer side** — a fixed-name local socket (`<state>/socks/daemon.sock`
//!   on Unix; a namespaced pipe on Windows). A producer connects, streams
//!   one CLI **request** line followed by that request's CLI **response**
//!   lines (newline-delimited JSON, no ack), then closes. `interprocess`
//!   inserts no framing of its own, so the trailing `\n` is the only
//!   delimiter — the same wire shape as [`crate::websockets::mcp_listener`].
//! - **Consumer side** — an [`axum`] WebSocket server bound to the
//!   daemon's configured `address:port`, single root endpoint (`/`).
//!   Every client that connects immediately begins receiving future
//!   frames; it is a pure push channel (inbound messages are ignored
//!   except to notice the client closing).
//!
//! Each producer connection is assigned a fresh `id`. Its first item is
//! wrapped as the SDK [`ViewerRequest`] (`{id, value}`); every following
//! item as [`ViewerResponseItem`] (`{id, path_type, value}`), where
//! `path_type` is lifted off that connection's opening request. The `id`
//! lets a consumer demultiplex concurrent producer streams; `path_type`
//! tags each response with the command that produced it.
//!
//! Frames are validated through the SDK Viewer types when the command is
//! recognized and passed through raw otherwise (forward-compat). The
//! underlying items stay opaque [`serde_json::Value`]s on the wire, so it
//! stays
//! forward-compatible with command shapes it predates.

use std::path::{Path, PathBuf};

#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{ListenerOptions, Name};
use objectiveai_sdk::cli::command::{ViewerRequest, ViewerResponseItem};
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

/// Spawn the fan-in listener: bind the fixed-name local socket and, for
/// every producer connection, drive [`handle_feed`], teeing its wrapped
/// items onto `tx`. Detached and best-effort — any bind failure simply
/// means no producer socket; the daemon is otherwise unaffected.
pub fn spawn_socket_listener(tx: broadcast::Sender<String>, state_dir: PathBuf) {
    tokio::spawn(async move {
        // Ensure the socks dir exists for the Unix filesystem socket;
        // harmless on Windows (which uses a namespaced pipe name).
        let _ = tokio::fs::create_dir_all(socks_dir(&state_dir)).await;
        let Ok(name) = socket_name(&state_dir) else {
            return;
        };
        // `try_overwrite` clears a stale socket file left by a crashed
        // predecessor; the singleton daemon lock guarantees no live peer.
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
            tokio::spawn(handle_feed(conn, tx.clone()));
        }
    });
}

/// Serve one producer connection: read newline-delimited JSON items,
/// wrap the first as [`ViewerRequest`] and the rest as
/// [`ViewerResponseItem`] (carrying the request's `path_type`), and
/// broadcast each on `tx`. No writes back — the producer streams and
/// closes with no ack. EOF ends the task.
async fn handle_feed(conn: LocalSocketStream, tx: broadcast::Sender<String>) {
    let (read_half, _write_half) = tokio::io::split(conn);
    let mut reader = BufReader::new(read_half);
    let id = uuid::Uuid::new_v4().to_string();
    // `None` until the opening request is read; then holds its `path`.
    let mut path: Option<String> = None;
    let mut line = String::new();
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
                // First item = the CLI request. Lift its `path_type`
                // (the shared command-path string) to tag every response.
                let p = value
                    .get("path_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                path = Some(p);
                // Wrap as a `ViewerRequest` ({id, value}). Validate
                // through the SDK type when the command is known; fall
                // back to the raw envelope for forward-compat with
                // commands this binary predates.
                let envelope = serde_json::json!({ "id": id.clone(), "value": value });
                match serde_json::from_value::<ViewerRequest>(envelope.clone()) {
                    Ok(vr) => serde_json::to_string(&vr),
                    Err(_) => Ok(envelope.to_string()),
                }
            }
            Some(p) => {
                // Wrap as a `ViewerResponseItem` ({id, path_type, value}),
                // same validate-or-passthrough treatment.
                let envelope = serde_json::json!({
                    "id": id.clone(),
                    "path_type": p.clone(),
                    "value": value,
                });
                match serde_json::from_value::<ViewerResponseItem>(envelope.clone()) {
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

/// Producer/test helper: connect to the daemon socket, stream `request`
/// then each of `responses` as newline-delimited JSON, and close. The
/// inverse of [`handle_feed`].
pub async fn feed_socket(
    state_dir: &Path,
    request: &serde_json::Value,
    responses: &[serde_json::Value],
) -> std::io::Result<()> {
    let name = socket_name(state_dir)?;
    let conn = LocalSocketStream::connect(name).await?;
    let (_read_half, mut write_half) = tokio::io::split(conn);
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
