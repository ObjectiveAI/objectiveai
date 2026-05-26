//! Transport selection + WebSocket transport helpers for the streaming endpoints.
//!
//! Each streaming endpoint (`/agent/completions`, `/vector/completions`,
//! etc.) lives behind a single `axum::routing::any(...)` route. The
//! handler inspects the request via the [`Transport`] extractor and
//! forks based on whether the client is actually upgrading to WS:
//!
//! - `Upgrade: websocket` header present → GET + WS handshake,
//!   response is a WebSocket text-frame stream (the `_ws` handler).
//! - Anything else (POST + JSON body, with or without `stream: true`)
//!   → the existing SSE handler. That handler returns `text/event-stream`
//!   when `body.stream` is true and a unary `application/json` when
//!   it's false — same dispatch the endpoint had before WS landed.
//!
//! WS wire protocol after the upgrade:
//!
//! - Client → server: one text frame with the JSON request body
//!   (`*CreateParams`), exactly the same shape the SSE branch
//!   deserializes from the POST body.
//! - Server → client: N text frames, one chunk per frame, JSON
//!   encoded — same `*Chunk` types each endpoint already emits.
//! - End of stream: server sends `Close(1000)`. No `[DONE]` sentinel.
//! - Error mid-stream: server sends one final text frame containing
//!   the JSON `ResponseError`, then `Close(1011)`.
//! - Body parse failure: error text frame, `Close(1003)`.
//!
//! Auth lives on the upgrade handshake (`Authorization` header), the
//! same place every other route validates it; the helpers below are
//! invoked only after the upgrade has been accepted.
//!
//! Stage 1 of #193; #194 tracks the migration.

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::extract::ws::{CloseCode, CloseFrame, Message, WebSocket, close_code};
use axum::http::request::Parts;
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use futures::stream::{SplitSink, SplitStream};
use objectiveai_sdk::error::ResponseError;
use serde::Serialize;
use tokio::sync::{Mutex, oneshot};

use crate::error::ResponseErrorExt;

/// Shared sender half of a split WebSocket, wrapped under a tokio
/// mutex so the send-side (chunk forwarder) and recv-side (notify
/// responder) can both write frames safely. Locks are short-lived —
/// only held across a single `send`.
pub type SharedSink = Arc<Mutex<SplitSink<WebSocket, Message>>>;

/// Per-WS-connection tracker of agent-completion `response_id`s
/// emitted by this stream. Populated on the send side as each chunk
/// flows out (via [`AgentCompletionIds`]) and read on the recv side
/// to validate incoming notify requests' `response_id`. Notifies
/// targeting an id not in this tracker are rejected with 404.
pub struct SessionTracker {
    ids: dashmap::DashSet<String>,
}

impl SessionTracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            ids: dashmap::DashSet::new(),
        })
    }

    /// Extend the tracker with every agent-completion id this chunk
    /// carries. Borrows into the chunk; no allocation beyond the
    /// `insert` itself.
    pub fn observe<C>(&self, chunk: &C)
    where
        C: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds,
    {
        for id in chunk.agent_completion_ids() {
            self.ids.insert(id.to_string());
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }
}

/// Transport the client wants. Inferred from the request itself: an
/// `Upgrade: websocket` header → [`Transport::WebSocket`], anything
/// else → [`Transport::Sse`]. The SSE handler covers both
/// streamed-SSE and unary-collected responses internally (selected
/// by `body.stream`); we only need to detect an actual WS upgrade
/// here. POST + JSON for unary or SSE never carries `Upgrade`, so
/// it always falls to the SSE branch — which is what unary callers
/// expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Sse,
    WebSocket,
}

impl<S> FromRequestParts<S> for Transport
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let is_ws_upgrade = parts
            .headers
            .get(axum::http::header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);
        Ok(if is_ws_upgrade {
            Transport::WebSocket
        } else {
            Transport::Sse
        })
    }
}

/// Render a `400 Bad Request` response with the given message in a
/// JSON `ResponseError` envelope. Used by the transport dispatcher
/// when the client's combination of method + headers + body doesn't
/// match the transport they asked for.
pub fn bad_request(message: &str) -> Response {
    ResponseError {
        code: 400,
        message: serde_json::Value::String(message.to_string()),
    }
    .into_response()
}
use serde::de::DeserializeOwned;

/// Read exactly one text frame from `socket` and deserialize it as `T`.
///
/// Skips pings/pongs/binary frames silently — only a text frame is a
/// valid body. Returns a `ResponseError` describing the failure if
/// the peer closes early, sends something we can't parse, or sends a
/// non-text frame.
///
/// Caller is responsible for closing the socket on error (typically
/// via [`send_error_and_close`]).
pub async fn recv_body_frame<T: DeserializeOwned>(
    socket: &mut WebSocket,
) -> Result<T, ResponseError> {
    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => {
                return serde_json::from_str::<T>(text.as_str()).map_err(|e| ResponseError {
                    code: 400,
                    message: serde_json::Value::String(format!(
                        "failed to deserialize body frame: {e}"
                    )),
                });
            }
            Some(Ok(Message::Binary(_))) => {
                return Err(ResponseError {
                    code: 400,
                    message: serde_json::Value::String(
                        "expected text body frame, got binary".into(),
                    ),
                });
            }
            // Library handles ping/pong automatically; ignore if surfaced.
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(Message::Close(_))) | None => {
                return Err(ResponseError {
                    code: 400,
                    message: serde_json::Value::String(
                        "peer closed before sending body".into(),
                    ),
                });
            }
            Some(Err(e)) => {
                return Err(ResponseError {
                    code: 400,
                    message: serde_json::Value::String(format!("websocket recv error: {e}")),
                });
            }
        }
    }
}

/// Send `err` as a single text frame, then close with `code`.
///
/// Failures to send are swallowed — the socket is being torn down
/// anyway, and the peer can only do one of the two no-ops (notice the
/// close, or notice nothing because they've already gone).
pub async fn send_error_and_close(socket: &mut WebSocket, err: &ResponseError, code: CloseCode) {
    let frame = serde_json::to_string(err).unwrap_or_else(|_| String::from("{}"));
    let _ = socket.send(Message::Text(frame.into())).await;
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: "".into(),
        })))
        .await;
}

/// Close the socket with `Close(1011)` after sending the given
/// `ResponseError` as a text frame. Used when setup (e.g.
/// `create_streaming_handle_usage`) fails before any chunk has been
/// produced.
pub async fn fatal_setup_error(socket: &mut WebSocket, err: &ResponseError) {
    send_error_and_close(socket, err, close_code::ERROR).await;
}

/// Split-sink variant of [`fatal_setup_error`]. Used after the
/// socket has already been split (which is the order the WS
/// handlers now use so the reverse-attach guard can be built
/// before stream creation).
pub async fn fatal_setup_error_split(sink: &SharedSink, err: &ResponseError) {
    let frame = serde_json::to_string(err).unwrap_or_else(|_| String::from("{}"));
    {
        let mut guard = sink.lock().await;
        let _ = guard.send(Message::Text(frame.into())).await;
    }
    send_close_split(sink, close_code::ERROR).await;
}

// ────────────────────────────────────────────────────────────────────
// Split-sink variants. Used by `_ws` handlers after splitting the
// socket so the send-side (chunk forwarder) and recv-side (notify
// responder) can write through the same socket concurrently.
// ────────────────────────────────────────────────────────────────────

/// Send one chunk as a text frame. Caller observes the chunk into the
/// session tracker beforehand. Returns `Err(())` if the peer hung up.
pub async fn send_chunk_split<C: Serialize>(sink: &SharedSink, chunk: &C) -> Result<(), ()> {
    let json = match serde_json::to_string(chunk) {
        Ok(s) => s,
        Err(_) => return Ok(()), // chunk types are infallible to serialize in practice
    };
    let mut guard = sink.lock().await;
    let result = guard
        .send(Message::Text(json.into()))
        .await
        .map_err(|_| ());
    result
}

/// Send a `Close(code)` frame, ignoring any I/O error.
pub async fn send_close_split(sink: &SharedSink, code: CloseCode) {
    let mut guard = sink.lock().await;
    let _ = guard
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: "".into(),
        })))
        .await;
}

/// Per-WS-connection registry of outstanding
/// [`server_request::Request`](objectiveai_sdk::client_objectiveai_mcp::server_request::Request)s
/// the API has emitted and is awaiting a matching
/// [`server_response::Response`](objectiveai_sdk::client_objectiveai_mcp::server_response::Response)
/// for. Keys are the API-minted `id`; values are the oneshot the
/// awaiting future is parked on. The recv side of the WS drains
/// `server_response` frames, looks up `id`, and fulfills the oneshot
/// with the full response (status + headers + body).
pub type PendingRequests = Arc<
    dashmap::DashMap<
        String,
        oneshot::Sender<objectiveai_sdk::client_objectiveai_mcp::server_response::Response>,
    >,
>;

pub fn new_pending_requests() -> PendingRequests {
    Arc::new(dashmap::DashMap::new())
}

/// Reverse-attach handle for the API's MCP endpoint to forward proxy
/// traffic over an in-flight agent-completion WS. Holds both halves
/// of the per-connection state: the sink to write `server_request`
/// frames out, and the registry to park awaits for matching
/// `server_response` frames coming back.
#[derive(Clone)]
pub struct ReverseChannel {
    pub sink: SharedSink,
    pub pending: PendingRequests,
}

/// Process-wide registry of live [`ReverseChannel`]s keyed by an
/// opaque session id minted on WS upgrade. Populated by the `_ws`
/// handlers; consulted by the `/objectiveai-mcp/<session_id>` route
/// and by the agent-completion verification probe.
pub type ReverseChannelRegistry = Arc<dashmap::DashMap<String, ReverseChannel>>;

pub fn new_reverse_channel_registry() -> ReverseChannelRegistry {
    Arc::new(dashmap::DashMap::new())
}

/// Bundle of the things each `_ws` handler needs to wire up the
/// reverse-attach: the global [`ReverseChannelRegistry`] (so it can
/// insert/remove its session) plus the API's own listening port (so
/// the agent client can build a `http://127.0.0.1:<port>/objectiveai-mcp/<session>`
/// URL the proxy will dial).
#[derive(Clone)]
pub struct ReverseAttachConfig {
    pub registry: ReverseChannelRegistry,
    pub api_port: u16,
}

/// Arc-shareable handle the agent client uses to register per-agent
/// `ws_session_id`s against the current WS [`ReverseChannel`]. Many
/// ids may map to one channel — one CLI WS upgrade can serve a swarm
/// of N agents, each declaring `client_objectiveai_mcp` with its own
/// stable `ws_session_id` (minted fresh for new agents, recovered
/// from continuation for resuming ones). The owning
/// [`ReverseAttachGuard`] removes every registered id on drop.
pub struct ReverseAttachHandle {
    registry: ReverseChannelRegistry,
    channel: ReverseChannel,
    registered: std::sync::Mutex<Vec<String>>,
}

impl std::fmt::Debug for ReverseAttachHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self
            .registered
            .try_lock()
            .map(|g| g.len())
            .unwrap_or(usize::MAX);
        f.debug_struct("ReverseAttachHandle")
            .field("registered_count", &count)
            .finish_non_exhaustive()
    }
}

impl ReverseAttachHandle {
    /// Inserts `id -> channel` into the registry and tracks the id
    /// for cleanup when the owning guard drops. Calling with the same
    /// id twice is harmless (the registry just overwrites).
    pub fn register(&self, id: String) {
        self.registry
            .insert(id.clone(), self.channel.clone());
        self.registered.lock().unwrap().push(id);
    }
}

/// RAII guard for one CLI WS upgrade. Owns the registration handle;
/// when it drops, every id registered via the handle is removed from
/// the [`ReverseChannelRegistry`]. `Arc` clones of the handle may
/// outlive the guard (e.g. background usage-tracker tasks holding
/// onto a copy of the ctx) — they observe a drained registration
/// list and any further `register()` calls leak harmlessly until the
/// last `Arc` drops.
pub struct ReverseAttachGuard {
    handle: Arc<ReverseAttachHandle>,
}

impl ReverseAttachGuard {
    pub fn new(
        registry: ReverseChannelRegistry,
        sink: SharedSink,
        pending: PendingRequests,
    ) -> Self {
        let handle = Arc::new(ReverseAttachHandle {
            registry,
            channel: ReverseChannel { sink, pending },
            registered: std::sync::Mutex::new(Vec::new()),
        });
        Self { handle }
    }

    /// Returns the shared handle the agent client should stamp on
    /// the per-request `Context` so it can register ids from inside
    /// the swarm-iteration site.
    pub fn handle(&self) -> Arc<ReverseAttachHandle> {
        self.handle.clone()
    }
}

impl Drop for ReverseAttachGuard {
    fn drop(&mut self) {
        let ids = std::mem::take(&mut *self.handle.registered.lock().unwrap());
        for id in ids {
            self.handle.registry.remove(&id);
        }
    }
}

/// Register a oneshot under `request.id`, write the request as a
/// text frame, and return the receiver. The caller is responsible
/// for minting the id (and putting it on the request) and applying
/// a timeout (via `tokio::time::timeout`) on the await. On
/// connection drop the recv loop returns and pending oneshots are
/// dropped — receivers observe the close as `Err(RecvError)`.
pub async fn send_server_request(
    sink: &SharedSink,
    pending: &PendingRequests,
    request: objectiveai_sdk::client_objectiveai_mcp::server_request::Request,
) -> Result<
    oneshot::Receiver<objectiveai_sdk::client_objectiveai_mcp::server_response::Response>,
    (),
> {
    let id = request.id.clone();
    let (tx, rx) = oneshot::channel();
    pending.insert(id.clone(), tx);

    let frame = match serde_json::to_string(&request) {
        Ok(s) => s,
        Err(_) => {
            pending.remove(&id);
            return Err(());
        }
    };
    let mut guard = sink.lock().await;
    let send_result = guard.send(Message::Text(frame.into())).await;
    if send_result.is_err() {
        drop(guard);
        pending.remove(&id);
        return Err(());
    }
    Ok(rx)
}

/// Recv loop: drain the split stream, parse each text frame, and
/// dispatch based on shape.
///
/// - Frames that parse as
///   [`client_request::Request`](objectiveai_sdk::client_objectiveai_mcp::client_request::Request)
///   go through the notify pipeline: validate `response_id` against
///   the session tracker, dispatch to `notify_fn`, write back a
///   [`client_response::Response`](objectiveai_sdk::client_objectiveai_mcp::client_response::Response)
///   echoing the request `id`.
/// - Frames that parse as
///   [`server_response::Response`](objectiveai_sdk::client_objectiveai_mcp::server_response::Response)
///   are routed to the pending-request registry: the matching
///   oneshot is taken and fulfilled. Unknown `id` → log + drop.
/// - Frames that match neither shape are logged + dropped.
///
/// Returns when the recv half closes (peer hung up or close frame).
pub async fn recv_loop<F, Fut>(
    mut rx: SplitStream<WebSocket>,
    tracker: Arc<SessionTracker>,
    sink: SharedSink,
    pending: PendingRequests,
    notify_fn: F,
) where
    F: Fn(objectiveai_sdk::agent::completions::request::AgentCompletionNotifyParams) -> Fut
        + Send
        + Sync
        + 'static,
    Fut: std::future::Future<Output = Result<(), crate::agent::completions::Error>>
        + Send
        + 'static,
{
    use objectiveai_sdk::client_objectiveai_mcp::{
        client_request::{Payload as ClientPayload, Request as ClientRequest},
        client_response::Response as ClientResponse,
        server_response::Response as ServerResponse,
    };

    // Arc-wrap the notify dispatcher so each spawned dispatch task
    // can hold its own cheap clone. Required because we spawn notify
    // handling to keep the recv loop unblocked for server_response
    // frames the MCP endpoint is timing out on.
    let notify_fn = Arc::new(notify_fn);

    loop {
        let msg = match rx.next().await {
            Some(m) => m,
            None => {
                return;
            }
        };
        let text = match msg {
            Ok(Message::Text(t)) => {
                t
            }
            Ok(Message::Binary(_)) => {
                eprintln!("ignoring binary frame on streaming WS recv side");
                continue;
            }
            Ok(Message::Ping(_) | Message::Pong(_)) => continue,
            Ok(Message::Close(_)) => {
                return;
            }
            Err(e) => {
                eprintln!("streaming WS recv error: {e}");
                return;
            }
        };

        // Parse strategy: try client_request first (the discriminator
        // tag `type` distinguishes it from server_response — they
        // share the `id` field but differ everywhere else), then
        // server_response, then drop.
        if let Ok(request) = serde_json::from_str::<ClientRequest>(text.as_str()) {
            let ClientRequest { id, payload } = request;
            match payload {
                ClientPayload::AgentCompletionNotify(params) => {
                    // Spawn so the recv loop can immediately move on
                    // to the next frame — notify dispatch can be slow
                    // (it hits the agent's MCP connection) and we
                    // don't want it blocking server_response routing.
                    let tracker = tracker.clone();
                    let sink = sink.clone();
                    let notify_fn = notify_fn.clone();
                    tokio::spawn(async move {
                        let response: ClientResponse = if !tracker.contains(&params.response_id) {
                            ClientResponse::Error {
                                id,
                                code: 404,
                                message: serde_json::Value::String(format!(
                                    "response_id {:?} not from this stream",
                                    params.response_id
                                )),
                            }
                        } else {
                            match (notify_fn)(params).await {
                                Ok(()) => ClientResponse::Ok { id },
                                Err(e) => {
                                    let inner = ResponseError::from(&e);
                                    ClientResponse::Error {
                                        id,
                                        code: inner.code,
                                        message: inner.message,
                                    }
                                }
                            }
                        };
                        let frame = match serde_json::to_string(&response) {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        let mut guard = sink.lock().await;
                        let _ = guard.send(Message::Text(frame.into())).await;
                    });
                    continue;
                }
            }
        }

        if let Ok(response) = serde_json::from_str::<ServerResponse>(text.as_str()) {
            match pending.remove(&response.id) {
                Some((_, tx)) => {
                    let _ = tx.send(response);
                }
                None => {
                    eprintln!(
                        "dropping server_response for unknown id {:?}",
                        response.id
                    );
                }
            }
            continue;
        }

        eprintln!("dropping unparseable WS frame (matched neither client_request nor server_response)");
    }
}
