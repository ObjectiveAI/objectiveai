//! The daemon's laboratory surface: the `/laboratory` WebSocket route
//! plus the `laboratories.sock` local socket.
//!
//! Laboratory MANAGERS (`objectiveai-laboratory` processes — local or
//! remote, the daemon cannot tell and does not care) dial IN on
//! `/laboratory`. The connection's wire order is load-bearing:
//!
//! 1. The FIRST text frame is the [`Identify`] — who this laboratory
//!    is. Identity always PRECEDES authorization on this endpoint.
//! 2. The SECOND frame is the standard first-message `AuthEnvelope`
//!    (verified by [`crate::websockets::daemon_auth::authenticate`],
//!    demoted to second place here).
//! 3. Then the daemon sends [`ChannelRequest`]s and the manager
//!    answers [`ChannelResponse`]s, correlated by id.
//!
//! The set of live `/laboratory` connections IS the laboratory
//! registry: `laboratories list` snapshots it, and a disconnect
//! removes the laboratory (its in-flight forwards fail cleanly). The
//! CLI conduit reaches connected laboratories through
//! `laboratories.sock` ([`SocketRequest`]/[`SocketResponse`], one JSON
//! line each way per connection — the `mcp_listener` protocol shape).

use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{ListenerOptions, Name};
#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{
    ChannelRequest, ChannelResponse, Identify, SocketRequest, SocketResponse,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use crate::websockets::mcp_listener::socks_dir;

/// How long a forward waits for the manager's reply. Generous — tool
/// calls and 2 MiB transfer chunks ride this; the API layer above owns
/// the real deadlines.
const FORWARD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// One connected laboratory manager.
struct LabConnection {
    identify: Identify,
    /// Frames queued to the manager (drained by the connection's
    /// writer half).
    tx: mpsc::UnboundedSender<ChannelRequest>,
    /// In-flight forwards awaiting the manager's correlated reply.
    /// Dropped wholesale on disconnect, failing every waiter.
    pending: DashMap<String, oneshot::Sender<ChannelResponse>>,
}

/// The connected-laboratory registry, shared by the `/laboratory`
/// route (writers) and the socket + `laboratories list` (readers).
#[derive(Clone)]
pub struct LaboratoryRegistry {
    labs: Arc<DashMap<String, Arc<LabConnection>>>,
}

impl LaboratoryRegistry {
    pub fn new() -> Self {
        Self { labs: Arc::new(DashMap::new()) }
    }

    /// Identity snapshots of every connected laboratory.
    pub fn list(&self) -> Vec<Identify> {
        self.labs.iter().map(|e| e.identify.clone()).collect()
    }

    /// Forward one request to a connected laboratory and await its
    /// correlated reply.
    pub async fn forward(
        &self,
        laboratory_id: &str,
        headers: indexmap::IndexMap<String, String>,
        request: objectiveai_sdk::client_objectiveai_mcp::server_request::Payload,
    ) -> Result<objectiveai_sdk::client_objectiveai_mcp::server_response::Payload, String> {
        // Clone the Arc out; never hold a map guard across an await.
        let lab = match self.labs.get(laboratory_id) {
            Some(lab) => Arc::clone(&lab),
            None => return Err(format!("laboratory '{laboratory_id}' is not connected")),
        };
        let id = uuid::Uuid::new_v4().to_string();
        let (reply_tx, reply_rx) = oneshot::channel();
        lab.pending.insert(id.clone(), reply_tx);
        let sent = lab.tx.send(ChannelRequest { id: id.clone(), headers, payload: request });
        if sent.is_err() {
            lab.pending.remove(&id);
            return Err(format!("laboratory '{laboratory_id}' disconnected"));
        }
        match tokio::time::timeout(FORWARD_TIMEOUT, reply_rx).await {
            Ok(Ok(response)) => Ok(response.payload),
            Ok(Err(_)) => {
                // Pending map dropped — the manager disconnected.
                Err(format!("laboratory '{laboratory_id}' disconnected mid-request"))
            }
            Err(_) => {
                lab.pending.remove(&id);
                Err(format!("laboratory '{laboratory_id}' timed out"))
            }
        }
    }
}

/// `/laboratory`: upgrade, read the Identify frame, consume the auth
/// preamble (strictly second), register, pump until disconnect.
pub(crate) async fn laboratory_handler(
    axum::extract::State(state): axum::extract::State<
        crate::websockets::daemon_stream::DaemonWsState,
    >,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        // 1. Identity FIRST. Control frames are skipped like the auth
        // reader does; anything unparseable closes the connection.
        let identify = loop {
            match socket.recv().await {
                Some(Ok(axum::extract::ws::Message::Text(text))) => {
                    match serde_json::from_str::<Identify>(&text) {
                        Ok(identify) => break identify,
                        Err(_) => {
                            let _ = socket.send(axum::extract::ws::Message::Close(None)).await;
                            return;
                        }
                    }
                }
                Some(Ok(axum::extract::ws::Message::Close(_))) | Some(Err(_)) | None => return,
                Some(Ok(_)) => continue,
            }
        };
        // 2. Authorization SECOND (the standard preamble, verbatim).
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref())
            .await
        {
            return;
        }
        // 3. Register. A live entry under this id means either a stale
        // duplicate (the id lock should prevent one) or a reconnect
        // racing its own predecessor's teardown — the NEW connection
        // wins: displace the old entry (its pending waiters fail).
        let (tx, mut rx) = mpsc::unbounded_channel::<ChannelRequest>();
        let lab = Arc::new(LabConnection {
            identify: identify.clone(),
            tx,
            pending: DashMap::new(),
        });
        state.laboratories.labs.insert(identify.id.clone(), Arc::clone(&lab));

        // Pump: outbound requests + inbound correlated replies.
        loop {
            tokio::select! {
                queued = rx.recv() => match queued {
                    Some(request) => {
                        let Ok(frame) = serde_json::to_string(&request) else {
                            continue;
                        };
                        if socket
                            .send(axum::extract::ws::Message::Text(frame.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    // Registry entry displaced (reconnect race) — this
                    // connection is dead weight; close it out.
                    None => break,
                },
                received = socket.recv() => match received {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        let Ok(response) = serde_json::from_str::<ChannelResponse>(&text) else {
                            continue;
                        };
                        if let Some((_, waiter)) = lab.pending.remove(&response.id) {
                            let _ = waiter.send(response);
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => continue,
                },
            }
        }

        // Deregister — but only if the entry is still OURS (a reconnect
        // may have displaced it already).
        state
            .laboratories
            .labs
            .remove_if(&identify.id, |_, current| Arc::ptr_eq(current, &lab));
        // Dropping `lab.pending` (last Arc) fails all in-flight waiters.
    })
}

// ── laboratories.sock ────────────────────────────────────────────

/// The fixed local-socket name, `laboratories` in place of `daemon` —
/// see [`crate::websockets::daemon_stream::bind_socket_listener`].
#[cfg(unix)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    socks_dir(state_dir)
        .join("laboratories.sock")
        .to_fs_name::<GenericFilePath>()
}

#[cfg(windows)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state_dir.file_name().hash(&mut hasher);
    let state = hasher.finish();
    format!("objectiveai-{state:016x}-laboratories.sock").to_ns_name::<GenericNamespaced>()
}

/// Bind `laboratories.sock` (synchronously, so the daemon publishes its
/// lock only after the socket is listening — the producer-socket
/// convention).
pub fn bind_laboratories_socket_listener(
    state_dir: &Path,
) -> std::io::Result<interprocess::local_socket::tokio::Listener> {
    let _ = std::fs::create_dir_all(socks_dir(state_dir));
    let name = socket_name(state_dir)?;
    ListenerOptions::new()
        .name(name)
        .try_overwrite(true)
        .create_tokio()
}

/// Accept loop: one connection = one [`SocketRequest`] line → one
/// [`SocketResponse`] line.
pub fn serve_laboratories_socket_listener(
    listener: interprocess::local_socket::tokio::Listener,
    registry: LaboratoryRegistry,
) {
    tokio::spawn(async move {
        loop {
            let conn = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };
            tokio::spawn(handle_conn(conn, registry.clone()));
        }
    });
}

async fn handle_conn(conn: LocalSocketStream, registry: LaboratoryRegistry) {
    let (read_half, mut write_half) = tokio::io::split(conn);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return;
    }

    let response = match serde_json::from_str::<SocketRequest>(line.trim()) {
        Ok(SocketRequest::Forward { laboratory_id, headers, request }) => {
            match registry.forward(&laboratory_id, headers, request).await {
                Ok(response) => SocketResponse::Forwarded { response },
                Err(message) => SocketResponse::Error { message },
            }
        }
        Ok(SocketRequest::List) => SocketResponse::List { laboratories: registry.list() },
        Err(e) => SocketResponse::Error { message: format!("malformed request: {e}") },
    };

    let Ok(reply) = serde_json::to_string(&response) else {
        return;
    };
    let _ = write_half.write_all(reply.as_bytes()).await;
    let _ = write_half.write_all(b"\n").await;
    let _ = write_half.shutdown().await;
}

/// Client side: one request line → one response line against
/// `laboratories.sock`. A connect failure means the daemon is not
/// running (or predates this socket).
pub async fn call_laboratories_socket(
    state_dir: &Path,
    request: &SocketRequest,
) -> std::io::Result<SocketResponse> {
    let name = socket_name(state_dir)?;
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
    serde_json::from_str::<SocketResponse>(reply.trim())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
