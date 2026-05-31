//! GET `/objectiveai-mcp` — Streamable HTTP MCP notifications stream.
//!
//! Subscribes to the per-`(ws_session_id, mcp_session_id)` broadcast
//! and emits standard MCP `notifications/<kind>/list_changed`
//! JSON-RPC envelopes whenever the CLI pushes one up over its
//! `client_request::Payload::McpListChanged`.

use super::listeners::McpListenerRegistry;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{
    IntoResponse, Response,
    sse::{Event, KeepAlive, Sse},
};
use futures::stream::StreamExt;
use objectiveai_sdk::client_objectiveai_mcp::client_request::McpListChangedKind;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;

/// SSE keepalive cadence for the GET notifications stream.
const SSE_KEEP_ALIVE: Duration = Duration::from_secs(15);

/// GET `/objectiveai-mcp`: open the per-MCP-session SSE notifications
/// stream the proxy subscribes to for
/// `notifications/{tools,resources}/list_changed`. Requires an
/// `Mcp-Session-Id` request header to identify which upstream MCP
/// connection's events to forward; without one we 400.
///
/// The stream emits standard MCP-spec JSON-RPC envelopes as `data:`
/// frames:
///
/// ```text
/// data: {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
/// ```
///
/// `KeepAlive` pings every [`SSE_KEEP_ALIVE`] hold the stream open
/// during quiet periods. When the last receiver hangs up the
/// stream's drop guard calls [`McpListenerRegistry::gc`].
pub async fn handle_get_sse(
    session_id: String,
    listeners: McpListenerRegistry,
    headers: HeaderMap,
) -> Response {
    let mcp_session_id = match headers
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Mcp-Session-Id header is required on GET /objectiveai-mcp",
            )
                .into_response();
        }
    };

    let rx = listeners.subscribe(&session_id, &mcp_session_id);

    // Wrap in a drop-guard so the registry GC fires when the
    // subscriber hangs up. `BroadcastStream` itself drops the
    // receiver when iteration stops, but it doesn't know about our
    // registry — we do the call here.
    struct GcGuard {
        listeners: McpListenerRegistry,
        ws_session_id: String,
        mcp_session_id: String,
    }
    impl Drop for GcGuard {
        fn drop(&mut self) {
            self.listeners.gc(&self.ws_session_id, &self.mcp_session_id);
        }
    }
    let gc = GcGuard {
        listeners,
        ws_session_id: session_id,
        mcp_session_id,
    };

    let stream = BroadcastStream::new(rx).filter_map(move |item: Result<
        McpListChangedKind,
        tokio_stream::wrappers::errors::BroadcastStreamRecvError,
    >| {
        // Keep the gc guard alive for the entire stream lifetime by
        // closing over it. The closure is owned by `filter_map`, which
        // is owned by the SSE stream, which is owned by the response;
        // it drops when the client disconnects.
        let _ = &gc;
        async move {
            let kind = item.ok()?;
            let value = serde_json::json!({
                "jsonrpc": "2.0",
                "method": kind.method(),
            });
            let json = serde_json::to_string(&value).ok()?;
            Some(Ok::<_, Infallible>(Event::default().data(json)))
        }
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(SSE_KEEP_ALIVE))
        .into_response()
}
