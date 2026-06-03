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
use futures::stream::SplitStream;
use objectiveai_sdk::error::ResponseError;
use serde::Serialize;
use tokio::sync::oneshot;

use crate::error::ResponseErrorExt;

// The reverse-attach / session-tracker / pending-request types are
// now canonical in `crate::objectiveai_mcp`. The `pub use` shims
// keep `crate::streaming_ws::SharedSink`,
// `crate::streaming_ws::SessionTracker`, etc. resolving for every
// existing call site in the api — the underlying type IS the
// objectiveai_mcp one.
pub use crate::objectiveai_mcp::{
    PendingRequests, ReverseAttachConfig, ReverseAttachGuard,
    ReverseAttachHandle, ReverseChannel, ReverseChannelRegistry,
    SessionTracker, SharedSink, new_pending_requests,
    new_reverse_channel_registry,
};

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

/// Split-sink variant. Used after the socket has already been split
/// (which is the order the WS handlers now use so the reverse-attach
/// guard can be built before stream creation). Closes the socket
/// with `Close(1011)` after sending `err` as a text frame; used when
/// setup fails before any chunk has been produced.
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

// PendingRequests, ReverseChannel, ReverseChannelRegistry,
// ReverseAttachConfig, ReverseAttachGuard, ReverseAttachHandle and
// the `new_*` constructors are re-exported at the top of this file
// from `crate::objectiveai_mcp`. `send_server_request` (used by the
// MCP route layer) also lives there as `objectiveai_mcp::send`.

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
    mcp_listeners: crate::objectiveai_mcp::McpListenerRegistry,
    attach_handle: Arc<ReverseAttachHandle>,
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
                // McpListChanged dispatch: fan out to every
                // (response_id, mcp_kind) keyed under this WS. The
                // GET-SSE subscriber for whichever agent's
                // response_id is currently active will receive the
                // event; idle keys publish to a no-op broadcast. The
                // fan handles reconnect-across-continuations — the
                // same upstream MCP can publish to multiple
                // response_ids that share the WS, only the live
                // subscriber gets the event.
                ClientPayload::McpListChanged(change) => {
                    for ws_session_id in attach_handle.registered_ids() {
                        mcp_listeners.publish(
                            &ws_session_id,
                            &change.mcp_kind,
                            change.kind,
                        );
                    }
                    let response = ClientResponse::Ok { id };
                    let frame = match serde_json::to_string(&response) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let sink = sink.clone();
                    tokio::spawn(async move {
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
