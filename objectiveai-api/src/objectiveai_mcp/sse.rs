//! GET on a per-MCP route — Streamable HTTP MCP notifications stream.
//!
//! Subscribes to the per-`(response_id, McpKind)` broadcast and
//! emits standard MCP `notifications/<kind>/list_changed` JSON-RPC
//! envelopes whenever the CLI pushes one up over its
//! `client_request::Payload::McpListChanged`.

use super::listeners::McpListenerRegistry;
use axum::http::HeaderMap;
use axum::response::{
    IntoResponse, Response,
    sse::{Event, KeepAlive, Sse},
};
use futures::stream::StreamExt;
use objectiveai_sdk::client_objectiveai_mcp::McpKind;
use objectiveai_sdk::client_objectiveai_mcp::client_request::McpListChangedKind;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;

/// SSE keepalive cadence for the GET notifications stream.
const SSE_KEEP_ALIVE: Duration = Duration::from_secs(15);

/// GET on a per-MCP route: open the SSE notifications stream the
/// proxy subscribes to for `notifications/{tools,resources}/list_changed`
/// on the upstream identified by [`McpKind`]. Routing inside the
/// API already happened (response_id → reverse channel; path →
/// `McpKind`) before this handler runs.
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
    response_id: String,
    mcp_kind: McpKind,
    listeners: McpListenerRegistry,
    _headers: HeaderMap,
) -> Response {
    let rx = listeners.subscribe(&response_id, &mcp_kind);

    // Wrap in a drop-guard so the registry GC fires when the
    // subscriber hangs up. `BroadcastStream` itself drops the
    // receiver when iteration stops, but it doesn't know about our
    // registry — we do the call here.
    struct GcGuard {
        listeners: McpListenerRegistry,
        response_id: String,
        mcp_kind: McpKind,
    }
    impl Drop for GcGuard {
        fn drop(&mut self) {
            self.listeners.gc(&self.response_id, &self.mcp_kind);
        }
    }
    let gc = GcGuard {
        listeners,
        response_id,
        mcp_kind,
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
